
use crate::{
    gui::prelude::*,
    util::hex_color::hex_color,
    client::{
        gui_blocks::fade::fade,
        menu::MENU_BACKGROUND,
    },
};
use std::{
    time::Duration,
    collections::VecDeque,
};
use graphics::prelude::*;


#[derive(Debug)]
pub struct GuiChat {
    lines: VecDeque<GuiChatLine>,
}

#[derive(Debug)]
struct GuiChatLine {
    text_block: GuiTextBlock<true>,
    added: Duration, // TODO: make some sort of epoch time newtype?
}

impl GuiChat {
    pub fn new() -> Self {
        GuiChat {
            lines: VecDeque::new(),
        }
    }

    pub fn add_line(&mut self, line: String, ctx: &GuiGlobalContext) {
        self.lines.push_back(GuiChatLine {
            text_block: GuiTextBlock::new(&GuiTextBlockConfig {
                text: &line,
                font: ctx.assets.font,
                logical_font_size: 16.0,
                color: hex_color(0xfbfbfbff),
                h_align: HAlign::Left,
                v_align: VAlign::Top,
            }),
            added: ctx.time_since_epoch,
        });
    }

    pub fn gui<'a>(&'a mut self, limit: bool) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        let lines = if limit {
            self.lines.range_mut(self.lines.len().saturating_sub(10)..)
        } else {
            self.lines.range_mut(0..)
        };

        logical_width(664.0,
            v_stack(0.0,
                lines.map(|chat_line| {
                    let line_gui = before_after(
                        (
                            solid(MENU_BACKGROUND),
                        ),
                        v_pad(2.0, 2.0,
                            h_margin(8.0, 8.0,
                                &mut chat_line.text_block
                            )
                        ),
                        (),
                    );
                    if limit {
                        GuiEither::A(fade(chat_line.added + Duration::from_secs(10), 1.0,
                            line_gui
                        ))
                    } else {
                        GuiEither::B(line_gui)
                    }
                })
                .collect::<Vec<_>>()
            )
        )
    }
}

pub fn make_chat_input_text_block(text: &str, blinker: bool, ctx: &GuiGlobalContext) -> GuiTextBlock<true> {
    let mut text = format!("saying: {}", text);
    if blinker {
        text.push('_');
    }

    GuiTextBlock::new(&GuiTextBlockConfig {
        text: &text,
        font: ctx.assets.font,
        logical_font_size: 16.0,
        color: hex_color(0xfbfbfbff),
        h_align: HAlign::Left,
        v_align: VAlign::Top,
    })
}
