
use crate::{
    gui::prelude::*,
    client::menu::{
        MenuGuiParams,
        MenuEffect,
        MenuEffectQueue,
    },
};


pub fn gui<'a>(args: MenuGuiParams<'a, '_>) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
    align([0.5, 0.0],
        logical_width(400.0,
            v_stack(0.0, (
                logical_height(40.0, gap()),
                &mut args.resources.options_menu_title_text,
                logical_height(22.0, gap()),
                h_align(0.5,
                    h_stack_auto(20.0, (
                        logical_width(300.0,
                            v_stack(8.0, (
                                args.resources.options_day_night_button.gui(args.ctx.global(), |s| &mut s.day_night),
                                args.resources.options_fog_button.gui(args.ctx.global(), |s| &mut s.fog),
                            ))
                        ),
                        logical_width(300.0,
                            v_stack(8.0, (
                                args.resources.options_load_dist_outline_button.gui(args.ctx.global(), |s| &mut s.load_dist_outline),
                                args.resources.options_chunk_outline_button.gui(args.ctx.global(), |s| &mut s.chunk_outline),
                            ))
                        ),
                    ))
                ),
                logical_height(32.0, gap()),
                args.resources.options_done_button.gui(on_options_done_click(&args.resources.effect_queue)),
            ))
        )
    )
}

fn on_options_done_click<'a>(effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {
        effect_queue.borrow_mut().push_back(MenuEffect::PopMenu);
    }
}
