
use crate::{
    asset::{
    	resource_pack::ResourcePack,
    	localization::Localization,
    },
	gui::{
		*,
		blocks::{
            *,
            simple_gui_block::SimpleGuiBlock,
        },
	},
	util::hex_color::hex_color,
};
use graphics::{
	Renderer,
	frame_content::{
		HAlign,
		VAlign,
	},
};
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


#[derive(Debug)]
struct GuiPrintOnClickBlock<'s>(&'s str);

impl<'a, 's> GuiNode<'a> for SimpleGuiBlock<GuiPrintOnClickBlock<'s>> {
    fn blocks_cursor(&self, _: GuiSpatialContext<'a>) -> bool { false }

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {
        if !hits { return }
        if !ctx.cursor_in_area(0.0, self.size) { return }
        if button != MouseButton::Left { return }

        println!("{}", self.inner.0);
    }
}


pub struct MainMenu {
    title: GuiTextBlock,
	version_text: GuiTextBlock,
    uncopyright_text: GuiTextBlock,
    singleplayer_button: MenuButton,
    multiplayer_button: MenuButton,
    mods_button: MenuButton,
    options_button: MenuButton,
}

pub struct MenuButton {
    text: GuiTextBlock,
    print_on_click: String,
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
            print_on_click: self.text.to_owned(),
        }
    }
}

impl MenuButton {
    pub fn gui<'a>(
        &'a mut self,
        _ctx: &'a GuiWindowContext<'a>,
    ) -> impl GuiBlock<'a, DimParentSets, DimChildSets>
    {
        logical_height(40.0,
            layer((
                GuiButtonBgBlock,
                &mut self.text,
                GuiPrintOnClickBlock(&self.print_on_click),
            )),
        )
    }
}

impl MainMenu {
	pub fn new(
		_renderer: &Renderer,
		resources: &ResourcePack,
		lang: &Localization,
	) -> Self
	{
        let title = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "minecraft lol",
            font: resources.font,
            logical_font_size: 64.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            wrap: false,
        });
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
		MainMenu {
            title,
			version_text,
			uncopyright_text,
            singleplayer_button,
            multiplayer_button,
            mods_button,
            options_button,
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
                    v_align(0.0,
                        v_stack(8.0, (
                            logical_height(200.0,
                                &mut self.title,
                            ),
                            self.singleplayer_button.gui(ctx),
                            self.multiplayer_button.gui(ctx),
                            self.mods_button.gui(ctx),
                            self.options_button.gui(ctx),
                        )),
                    ),
                ),
            ),
		))
	}
}

impl GuiStateFrame for MainMenu {
	fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: &'a GuiWindowContext<'a>,
        mut visitor: GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
		let ((), (), sized) = self
			.gui(ctx)
			.size(
				ctx.spatial.global,
				ctx.size.w as f32,
				ctx.size.h as f32,
				ctx.scale,
			);
		sized.visit_nodes(&mut visitor, forward);
    }
}
