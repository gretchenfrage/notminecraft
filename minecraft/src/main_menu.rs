
use crate::{
    asset::{
    	resource_pack::ResourcePack,
    	localization::Localization,
    },
	gui::{
		*,
		blocks::{
            *,
            mc::*,
            simple_gui_block::{
                SimpleGuiBlock,
                never_blocks_cursor_impl,
            },
        },
	},
	util::hex_color::hex_color,
    basic_demo::BasicDemo,
    singleplayer::Singleplayer,
};
use graphics::{
	Renderer,
	frame_content::{
		HAlign,
		VAlign,
	},
};
use std::{
    fmt::{self, Formatter, Debug},
    any::type_name,
};
use rand::thread_rng;
use vek::*;


#[derive(Debug)]
struct GuiButtonBgBlock;

impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for GuiButtonBgBlock {
    type Sized = GuiButtonBgBlockSized;

    fn size(
        self,
        _ctx: &GuiGlobalContext<'a>,
        w: f32,
        h: f32,
        scale: f32,
    ) -> ((), (), Self::Sized)
    {
        let sized = GuiButtonBgBlockSized {
            size: Extent2 { w, h },
            scale,
        };
        ((), (), sized)
    }
}

#[derive(Debug)]
struct GuiButtonBgBlockSized {
    size: Extent2<f32>,
    scale: f32,
}

impl<'a> SizedGuiBlock<'a> for GuiButtonBgBlockSized {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let highlight = visitor.ctx.cursor_pos
            .map(|pos|
                pos.x >= 0.0
                && pos.y >= 0.0
                && pos.x <= self.size.w
                && pos.y <= self.size.h
            )
            .unwrap_or(false);
        let images =
            if highlight { &visitor.ctx.resources().menu_button_highlight }
            else { &visitor.ctx.resources().menu_button };
        let ((), (), inner_sized) = tile_9(
            images,
            [400.0, 40.0],
            2.0 / 20.0,
            3.0 / 20.0,
            2.0 / 200.0,
            2.0 / 200.0,
        )
            .size(&visitor.ctx.global, self.size.w, self.size.h, self.scale);
        inner_sized.visit_nodes(visitor, forward);
    }
}

/*
#[derive(Debug)]
struct GuiPrintOnClickBlock<'s>(&'s str);

impl<'a, 's> GuiNode<'a> for SimpleGuiBlock<GuiPrintOnClickBlock<'s>> {
    never_blocks_cursor_impl!();

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {
        if !hits { return }
        if !ctx.cursor_in_area(0.0, self.size) { return }
        if button != MouseButton::Left { return }

        info!("{}", self.inner.0);
    }
}*/


struct GuiRunOnClickBlock<F>(F);

impl<
    'a,
    F: for<'r, 's> FnOnce(&'r GuiGlobalContext<'s>),
> GuiNode<'a> for SimpleGuiBlock<GuiRunOnClickBlock<F>>
{
    never_blocks_cursor_impl!();

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {
        if !hits { return }
        if !ctx.cursor_in_area(0.0, self.size) { return }
        if button != MouseButton::Left { return }

        (self.inner.0)(ctx.global)
    }
}

impl<F> Debug for GuiRunOnClickBlock<F> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&format!(
            "GuiRunOnClickBlock<{}>(..)",
            type_name::<F>(),
        ))
    }
}

#[derive(Debug)]
pub struct ScrollScaleChanger;

impl<'a> GuiNode<'a> for SimpleGuiBlock<ScrollScaleChanger>
{
    never_blocks_cursor_impl!();

    fn on_cursor_scroll(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        amount: ScrolledAmount,
    ) {
        trace!("on cursor scroll");

        if !hits { return }
        if !ctx.cursor_in_area(0.0, self.size) { return }

        let amount = amount.to_pixels(16.0).y;
        let scale = self.scale * f32::powf(1.01, amount);

        ctx.global.event_loop.borrow_mut().set_scale(scale);
    }
}


#[derive(Debug)]
pub struct MainMenu {
    title: GuiTitleBlock,
	version_text: GuiTextBlock,
    uncopyright_text: GuiTextBlock,

    /*
    singleplayer_button: MenuButton,
    multiplayer_button: MenuButton,
    mods_button: MenuButton,
    options_button: MenuButton,
    */
    basic_demo_button: MenuButton,
    singleplayer_button: MenuButton,
    exit_game_button: MenuButton,
    
    splash_text: GuiSplashText,
}

#[derive(Debug)]
pub struct MenuButton {
    text: GuiTextBlock,
}

pub struct MenuButtonBuilder<'a> {
    pub text: &'a str,
}

impl<'a> MenuButtonBuilder<'a> {
    pub fn new(text: &'a str) -> Self {
        MenuButtonBuilder {
            text,
        }
    }

    pub fn build(self, resources: &ResourcePack) -> MenuButton {
        let text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: self.text,
            font: resources.font,
            logical_font_size: 16.0,
            color: hex_color(0xE0E0E0FF),
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            wrap: false,
        });
        MenuButton {
            text,
        }
    }
}



impl MenuButton {
    pub fn gui<'a, F>(
        &'a mut self,
        on_click: F,
    ) -> impl GuiBlock<'a, DimParentSets, DimChildSets>
    where
        F: for<'r, 's> FnOnce(&'r GuiGlobalContext<'s>),
    {
        logical_height(40.0,
            layer((
                GuiButtonBgBlock,
                &mut self.text,
                GuiRunOnClickBlock(on_click),
            )),
        )
    }
}

impl MainMenu {
	pub fn new(
		renderer: &Renderer,
		resources: &ResourcePack,
		lang: &Localization,
	) -> Self
	{
        let title = GuiTitleBlock::new(renderer, &mut thread_rng());
		let version_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &lang.menu_version,
			font: resources.font,
			logical_font_size: 16.0,
			color: hex_color(0x505050FF),
			h_align: HAlign::Left,
			v_align: VAlign::Top,
			wrap: true,
		});
		let uncopyright_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &lang.menu_uncopyright,
			font: resources.font,
			logical_font_size: 16.0,
			color: Rgba::white(),
			h_align: HAlign::Right,
			v_align: VAlign::Bottom,
			wrap: true,
		});
        /*
        let singleplayer_button = MenuButtonBuilder
            ::new(&lang.menu_singleplayer)
            .build(resources);
        let multiplayer_button = MenuButtonBuilder
            ::new(&lang.menu_multiplayer)
            .build(resources);
        let mods_button = MenuButtonBuilder
            ::new(&lang.menu_mods)
            .build(resources);
        let options_button = MenuButtonBuilder
            ::new(&lang.menu_options)
            .build(resources);
        */
        let basic_demo_button = MenuButtonBuilder
            ::new("Basic Demo")
            .build(resources);
        let singleplayer_button = MenuButtonBuilder
            ::new(&lang.menu_singleplayer)
            .build(resources);
        let exit_game_button = MenuButtonBuilder
            ::new("Quit")
            .build(resources);
        let splash_text = GuiSplashText::new();
		MainMenu {
            title,
			version_text,
			uncopyright_text,
            /*
            multiplayer_button,
            mods_button,
            options_button,
            */
            basic_demo_button,
            singleplayer_button,
            exit_game_button,
            splash_text,
		}
	}

	fn gui<'a>(
		&'a mut self,
		ctx: &'a GuiWindowContext,
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		layer((
			modify(Rgba::new(0.25, 0.25, 0.25, 1.0),
                tile_image(&ctx.resources().menu_bg, 64.0)
            ),
			margin(4.0, 4.0, 4.0, 4.0,
                layer((
                    &mut self.version_text,
                    &mut self.uncopyright_text,
                ))
            ),
            h_align(0.5,
                logical_width(400.0,
                    layer((
                        v_align(0.0,
                            v_stack(8.0, (
                                logical_height(200.0,
                                    align(0.5,
                                        &self.title,
                                    ),
                                ),
                                /*
                                self.singleplayer_button.gui(ctx),
                                self.multiplayer_button.gui(ctx),
                                self.mods_button.gui(ctx),
                                self.options_button.gui(ctx),
                                */
                                self.basic_demo_button
                                    .gui(on_basic_demo_click),
                                self.singleplayer_button
                                    .gui(on_singleplayer_click),
                                self.exit_game_button
                                    .gui(on_exit_game_click),
                            )),
                        ),
                        v_align(0.0,
                            logical_height(200.0,
                                &mut self.splash_text,
                            ),
                        ),
                    )),
                ),
            ),
            ScrollScaleChanger,
		))
	}
}

fn on_basic_demo_click(ctx: &GuiGlobalContext) {
    ctx.push_state_frame(BasicDemo::new(
        ctx.game,
        &ctx.renderer.borrow(),
    ));
}

fn on_singleplayer_click(ctx: &GuiGlobalContext) {
    ctx.push_state_frame(Singleplayer::new(
        ctx.game,
    ));
}

fn on_exit_game_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}

impl GuiStateFrame for MainMenu {
	impl_visit_nodes!();

    fn update(&mut self, _ctx: &GuiWindowContext, elapsed: f32) {
        self.title.update(elapsed);
        self.splash_text.update(elapsed);
    }
}
