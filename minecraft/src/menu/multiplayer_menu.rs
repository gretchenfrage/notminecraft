
use crate::{
    gui::prelude::*,
    util::hex_color::hex_color,
    client::Client,
};
use graphics::prelude::*;
use vek::*;


#[derive(Debug)]
pub struct MultiplayerMenu {
    title_text: GuiTextBlock<true>,
    info_text_1: GuiTextBlock<true>,
    info_text_2: GuiTextBlock<true>,
    connect_button: MenuButton,
    cancel_button: MenuButton,

    address: String,
    address_text_block: GuiTextBlock<false>,
    address_blinker: bool,
    address_blinker_time: f32,
}

impl MultiplayerMenu {
    pub fn new(ctx: &GuiGlobalContext) -> Self
    {
        let title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.multiplayer_title,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Top,
        });
        let info_text_1 = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Not Minecraft Beta 1.0.2 multiplayer is yes! \
                   The default port is 35565.",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xa0a0a0ff),
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });
        let info_text_2 = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Enter the address of a server to connect to it:",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xa0a0a0ff),
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });
        let connect_button = menu_button(&ctx.assets.lang.multiplayer_connect)
            .build(&ctx.assets);
        let cancel_button = menu_button(&ctx.assets.lang.gui_cancel)
            .build(&ctx.assets);
        let address_text_block = make_address_text_block("", true, ctx);
        MultiplayerMenu {
            title_text,
            info_text_1,
            info_text_2,
            connect_button,
            cancel_button,
            address: String::new(),
            address_text_block,
            address_blinker: true,
            address_blinker_time: 0.0,
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        layer((
            modify(Rgba::new(0.25, 0.25, 0.25, 1.0),
                tile_image(&ctx.assets().menu_bg, 64.0)
            ),
            align(0.5,
                logical_size([562.0, 400.0],
                    v_align(0.0,
                        v_stack(0.0, (
                            &mut self.title_text,
                            logical_height(64.0, gap()),
                            &mut self.info_text_1,
                            logical_height(38.0, gap()),
                            &mut self.info_text_2,
                            logical_height(26.0, gap()),
                            h_align(0.5,
                                logical_size([404.0, 44.0],
                                    layer((
                                        AddressBoxBackground,
                                        h_margin(10.0, 10.0,
                                            align([0.0, 0.5],
                                                &mut self.address_text_block
                                            )
                                        ),
                                    ))
                                )
                            ),
                            logical_height(58.0, gap()),
                            h_align(0.5,
                                logical_width(400.0,
                                    self.connect_button.gui(on_connect_click(&self.address))
                                )
                            ),
                            logical_height(9.0, gap()),
                            h_align(0.5,
                                logical_width(400.0,
                                    self.cancel_button.gui(on_cancel_click)
                                )
                            ),
                        ))
                    )
                )
            )
        ))
    }
}

impl GuiStateFrame for MultiplayerMenu {
    impl_visit_nodes!();

    fn on_character_input(&mut self, ctx: &GuiWindowContext, c: char) {
        if c.is_control() {
            if c == '\u{8}' {
                // backspace
                self.address.pop();
            } else {
                trace!(?c, "ignoring unknown control character");
                return;
            }
        } else {
            self.address.push(c);
        }
        self.address_text_block = make_address_text_block(&self.address, self.address_blinker, ctx.global())
    }

    fn on_key_press_semantic(&mut self, ctx: &GuiWindowContext, key: VirtualKeyCode) {
        if key == VirtualKeyCode::Return {
            on_connect_click(&self.address)(ctx.global())
        } else if key == VirtualKeyCode::V && ctx.global().is_command_key_pressed() {
            self.address.push_str(&ctx.global().clipboard.get());
            self.address_text_block = make_address_text_block(&self.address, self.address_blinker, ctx.global())
        }
    }

    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {
        const BLINKEY: f32 = 1.0 / 3.0;

        self.address_blinker_time += elapsed;
        self.address_blinker_time %= BLINKEY * 2.0;
        let new_address_blinker = self.address_blinker_time < BLINKEY;
        if self.address_blinker != new_address_blinker {
            self.address_blinker = new_address_blinker;
            self.address_text_block = make_address_text_block(&self.address, self.address_blinker, ctx.global())
        }
    }
}

fn on_connect_click<'a>(address: &'a str) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |ctx| {
        ctx.pop_state_frame();
        ctx.push_state_frame(Client::connect(address, ctx));
    }
}

fn on_cancel_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}

fn make_address_text_block(address: &str, blinker: bool, ctx: &GuiGlobalContext) -> GuiTextBlock<false> {
    let mut address = address.to_string();
    if blinker {
        address.push('_');
    }
    GuiTextBlock::new(&GuiTextBlockConfig {
        text: &address,
        font: ctx.assets.font,
        logical_font_size: 16.0,
        color: hex_color(0xe0e0e0ff),
        h_align: HAlign::Left,
        v_align: VAlign::Center,
    })
}


/// GUI block for the address box background.
#[derive(Debug)]
struct AddressBoxBackground;

impl<'a> GuiNode<'a> for SimpleGuiBlock<AddressBoxBackground> {
    simple_blocks_cursor_impl!();

    fn draw(self, _: GuiSpatialContext<'a>, canvas: &mut Canvas2) {
        let border = 2.0 * self.scale;
        let border = Vec2::from(border);

        canvas.reborrow()
            .color(hex_color(0xa0a0a0ff))
            .draw_solid(self.size);
        canvas.reborrow()
            .translate(border)
            .color(Rgba::black())
            .draw_solid(self.size - border * 2.0);
    }
}
