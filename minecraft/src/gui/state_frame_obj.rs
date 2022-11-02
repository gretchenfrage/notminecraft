//! Object-safe way to access `GuiStateFrame`'s non-object-safe functionality.


use graphics::{
	frame_content::{
		FrameContent,
		FrameItem,
		Canvas2,
	},
	modifier::Modifier2,
};
use crate::gui::{
	context::{
		GuiWindowContext,
		GuiSpatialContext,
		MouseButton,
	},
	event::ScrolledAmount,
	node::{
		GuiNode,
		GuiVisitor,
		GuiVisitorTarget,
	},
	state_frame::GuiStateFrame,
};
use std::borrow::Cow;


// ==== trait ====

/// Object-safe trait to access `GuiStateFrame`'s non-object-safe
/// functionality. Blanket-impl'd for all `GuiStateFrame`.
///
/// Implements nodes functionality via `GuiStateFrame::visit_nodes`. As such,
/// provides a `GuiStateFrame` analogue to various methods of `GuiNode`. By
/// putting the dynamic dispatch boundary here, the compile can optimize GUI
/// component sizing, layout, and usage into a very efficient monomorphized
/// machine-code monolith.
pub trait GuiStateFrameObj: GuiStateFrame {
    fn on_cursor_move(&mut self, ctx: &GuiWindowContext);

    fn on_cursor_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    fn on_cursor_release(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    fn on_cursor_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
    );

    fn draw<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
        target: &mut FrameContent<'a>,
    );
}


trait CursorCallback<'a> {
	fn call<I: GuiNode<'a>>(
		&mut self,
		node: I,
		ctx: GuiSpatialContext,
		hits: bool,
	);
}


// ==== blanket impl ====

impl<T: GuiStateFrame> GuiStateFrameObj for T {
    fn on_cursor_move(&mut self, ctx: &GuiWindowContext) {
    	struct Callback;
    	impl<'a> CursorCallback<'a> for Callback {
    		fn call<I: GuiNode<'a>>(
				&mut self,
				node: I,
				ctx: GuiSpatialContext,
				hits: bool,
			) {
				node.on_cursor_move(ctx, hits);
			}
    	}
    	handle_cursor_event(self, ctx, Callback);
    }

    fn on_cursor_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    ) {
    	struct Callback(MouseButton);
    	impl<'a> CursorCallback<'a> for Callback {
    		fn call<I: GuiNode<'a>>(
				&mut self,
				node: I,
				ctx: GuiSpatialContext,
				hits: bool,
			) {
				node.on_cursor_click(ctx, hits, self.0);
			}
    	}
    	handle_cursor_event(self, ctx, Callback(button));
    }

    fn on_cursor_release(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    ) {
    	struct Callback(MouseButton);
    	impl<'a> CursorCallback<'a> for Callback {
    		fn call<I: GuiNode<'a>>(
				&mut self,
				node: I,
				ctx: GuiSpatialContext,
				hits: bool,
			) {
				node.on_cursor_release(ctx, hits, self.0);
			}
    	}
    	handle_cursor_event(self, ctx, Callback(button));
    }

    fn on_cursor_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
    ) {
    	struct Callback(ScrolledAmount);
    	impl<'a> CursorCallback<'a> for Callback {
    		fn call<I: GuiNode<'a>>(
				&mut self,
				node: I,
				ctx: GuiSpatialContext,
				hits: bool,
			) {
				node.on_cursor_scroll(ctx, hits, self.0);
			}
    	}
    	handle_cursor_event(self, ctx, Callback(amount));
    }

    fn draw<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
        target: &mut FrameContent<'a>,
    ) {
    	self
    		.visit_nodes(
                ctx,
    			GuiVisitor::new(
    				&mut DrawTarget::new(ctx.spatial, target),
    				ctx.spatial,
				),
    		);
    }
}


// ==== cursor node visitor target ====

struct CursorTarget<'c, F> {
    top: CursorTargetStackElem<'c>,
    under: Vec<CursorTargetStackElem<'c>>,
    unblocked: bool,
    callback: F,
}

#[derive(Copy, Clone)]
struct CursorTargetStackElem<'c> {
    ctx: GuiSpatialContext<'c>,
    unclipped: bool,
}

impl<'c> CursorTargetStackElem<'c> {
    fn relativize(&mut self, modifier: Modifier2) {
        match modifier {
            Modifier2::Transform(transform) => self.ctx.relativize(transform),
            Modifier2::Color(_) => (),
            Modifier2::Clip(clip) => {
                if let Some(pos) = self.ctx.cursor_pos {
                    self.unclipped = self.unclipped && clip.test(pos);
                }
            }
        }
    }
}

impl<'c, F> CursorTarget<'c, F> {
	fn new(ctx: GuiSpatialContext<'c>, callback: F) -> Self {
		CursorTarget {
			top: CursorTargetStackElem {
				ctx,
				unclipped: true,
			},
			under: Vec::new(),
			unblocked: true,
			callback,
		}
	}

	fn set_stack_len(&mut self, stack_len: usize) {
        assert!(stack_len <= self.under.len(), "stack_len too high");
        while self.under.len() > stack_len {
            self.top = self.under.pop().unwrap();
        }
    }
}

impl<'a, 'c, F: CursorCallback<'a>> GuiVisitorTarget<'a> for CursorTarget<'c, F> {
	fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
        self.set_stack_len(stack_len);
        self.under.push(self.top);
        self.top.relativize(modifier);
    }

    fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I) {
    	self.set_stack_len(stack_len);
                
        let ctx = self.top.ctx;
        let hits = self.unblocked && self.top.unclipped;

        if let Some(pos) = ctx.cursor_pos {
            self.unblocked = self.unblocked && !node.blocks_cursor(ctx, pos);
        }

        self.callback.call(node, ctx, hits);
    }

    fn push_debug_tag(&mut self, stack_len: usize, _: Cow<'static, str>) {
        self.set_stack_len(stack_len);
        self.under.push(self.top);
    }
}

fn handle_cursor_event<'a, F: CursorCallback<'a>, T: GuiStateFrame>(
	frame: &'a mut T,
	ctx: &'a GuiWindowContext,
	callback: F,
) {
	frame
		.visit_nodes(
			ctx,
			GuiVisitor::new(
				&mut CursorTarget::new(ctx.spatial, callback),
				ctx.spatial,
			),
		);
}


// ==== draw node visitor target ====

#[derive(Debug)]
struct DrawTarget<'a, 'c> {
	top: GuiSpatialContext<'c>,
    under: Vec<GuiSpatialContext<'c>>,
    frame_content: &'c mut FrameContent<'a>,
}

impl<'a, 'c> DrawTarget<'a, 'c> {
	fn new(
		ctx: GuiSpatialContext<'c>,
		frame_content: &'c mut FrameContent<'a>,
	) -> Self
	{
		DrawTarget {
			top: ctx,
			under: Vec::new(),
			frame_content,
		}
	}

	fn set_stack_len(&mut self, stack_len: usize) {
        assert!(stack_len <= self.under.len(), "stack_len too high");
        while self.under.len() > stack_len {
            self.top = self.under.pop().unwrap();
        }
    }   
}

impl<'a, 'c> GuiVisitorTarget<'a> for DrawTarget<'a, 'c> {
	fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
        self.set_stack_len(stack_len);

        self.under.push(self.top);
        if let Modifier2::Transform(transform) = modifier {
	        self.top.relativize(transform);
	    }

        self.frame_content.0.push((
        	self.under.len(),
        	FrameItem::PushModifier2(modifier),
        ));
    }

    fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I) {
    	self.set_stack_len(stack_len);

    	node.draw(self.top, &mut Canvas2 {
    		target: self.frame_content,
    		stack_len: self.under.len(),
    	});
    }

    fn push_debug_tag(&mut self, stack_len: usize, tag: Cow<'static, str>) {
        self.set_stack_len(stack_len);

        self.frame_content.0.push((
            self.under.len(),
            FrameItem::PushDebugTag(tag),
        ));
    }
}
