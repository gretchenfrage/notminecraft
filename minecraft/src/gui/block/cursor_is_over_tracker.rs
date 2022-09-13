
use crate::gui::{
    GuiNode,
    GuiContext,
    block::{
        GuiBlock,
        DimParentSets,
    },
};


struct TrackCursorIsOver<'v> {
    var: &'v mut bool,
}

impl<'v> GuiBlock<'v, DimParentSets, DimParentSets> for TrackCursorIsOver<'v> {
    type Sized = TrackCursorIsOverSized<'v>;

    fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
        let sized = TrackCursorIsOverSized {
            block: self,
            size: Extent2 { w, h },
            scale,
        };
        ((), (), sized)
    }
}

struct TrackCursorIsOverSized<'v> {
    block: TrackCursorIsOver<'v>,
    size: Extent2<f32>,
    scale: f32,
}

impl<'a, 'v> GuiNode<'a> for TrackCursorIsOverSized<'v> {
    fn clips_cursor(&self, _: Vec2<f32>) -> bool { false }

    fn on_cursor_change(self, ctx: &GuiContext) {
        let cursor_is_over = ctx
            .cursor
            .map(|cursor| cursor.unclipped
                && cursor.pos.x >= 0.0
                && cursor.pox.y >= 0.0
                && cursor.pos.x <= self.size.w
                && cursor.pos.y <= self.size.h
            )
            .unwrap_or(false);
        *self.block.var = cursor_is_over;
    }
}

/*
struct CursorIsOverTracker {
    pub cursor_is_over: bool,
}

impl CursorIsOverTracker {
    pub fn new() -> Self {
        CursorIsOverTracker {
            cursor_is_over: false,
        }
    }
}

impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a mut CursorIsOverTracker {
    type Sized = CursorIsOverTrackerSized<'a>;

    fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
        let sized = CursorIsOverTrackerSized {
            block: self,
            size: Extent2 { w, h },
            scale,
        };
        ((), (), sized)
    }
}

pub struct CursorIsOverTrackerSized<'a> {
    block: &'a mut CursorIsOverTracker,
    size: Extent2<f32>,
    scale: f32,
}

impl<'a> GuiNode<'a> for CursorIsOverTrackerSized<'a> {
    fn clips_cursor(&self, pos: Vec2<f32>) -> bool {
        pos.x >= 0.0
            && pos.y >= 0.0
            && pos.x <= self.size.w
            && pos.y <= self.size.h
    }
    /*
    fn on_cursor_click(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) -> CursorEventConsumed;

    fn on_cursor_scroll(self, ctx: &GuiContext, amount: ScrolledAmount) -> CursorEventConsumed;

    fn handle_input_event(self, _: &Renderer, event: InputEvent) {
        if let InputEvent::CursorMoved(pos) = event {
            self.block.cursor_is_over =
                pos.x >= 0.0
                && pos.y >= 0.0
                && pos.x <= self.size.w
                && pos.y <= self.size.h;
        }
    }
    */
}*/