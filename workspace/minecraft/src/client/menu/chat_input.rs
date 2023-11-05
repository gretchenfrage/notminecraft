
use crate::{
    gui::prelude::*,
    client::{
        menu::{
            MENU_BACKGROUND,
            MenuGuiParams,
        },
        gui_blocks::chat::make_chat_input_text_block,
    },
    util::secs_rem::secs_rem,
};

#[derive(Debug)]
pub struct ChatInput {
    pub t_preventer: bool,
    pub text: String,
    pub text_block: GuiTextBlock<true>,
    pub blinker: bool,
}

impl ChatInput {
    pub fn gui<'a>(
        &'a mut self,
        args: MenuGuiParams<'a, '_>,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
        v_align(1.0,
            v_stack(0.0, (
                h_align(0.0,
                    args.chat.take().unwrap().gui(false)
                ),
                min_height(80.0, 1.0,
                    h_margin(4.0, 4.0,
                        v_pad(4.0, 4.0,
                            before_after(
                                (
                                    solid(MENU_BACKGROUND),
                                ),
                                min_height(24.0, 1.0,
                                    h_margin(4.0, 4.0,
                                        v_pad(4.0, 4.0,
                                            &mut self.text_block,
                                        )
                                    )
                                ),
                                (),
                            )
                        )
                    )
                ),
            ))
        )
    }

    pub fn update(&mut self, ctx: &GuiGlobalContext) {
        self.t_preventer = false;

        let prev_blinker = self.blinker;
        self.blinker = secs_rem(ctx.time_since_epoch, 2.0 / 3.0) < 1.0 / 3.0;
        if self.blinker != prev_blinker {
            self.text_block = make_chat_input_text_block(&self.text, self.blinker, ctx);
        }
    }
}
