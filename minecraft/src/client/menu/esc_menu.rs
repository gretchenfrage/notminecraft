
use crate::{
    gui::prelude::*,
    client::{
        menu::{
            Menu,
            MenuGuiParams,
            MenuEffect,
            MenuEffectQueue,
            MenuResources,
        },
        InternalServer,
    },
};


pub fn gui<'a>(
    args: MenuGuiParams<'a, '_>,
    resources: &'a mut MenuResources,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
    align(0.5,
        logical_size([400.0, 320.0],
            v_align(0.0,
                v_stack(0.0, (
                    &mut resources.esc_menu_title_text,
                    logical_height(72.0, gap()),
                    resources.exit_menu_button.gui(on_exit_menu_click(&resources.effect_queue)),
                    logical_height(8.0, gap()),
                    resources.exit_game_button.gui(on_exit_game_click),
                    logical_height(8.0, gap()),
                    resources.open_to_lan_button.gui(on_open_to_lan_click(args.internal_server)),
                    logical_height(56.0 - 48.0, gap()),
                    resources.options_button.gui(on_options_click(&resources.effect_queue)),
                ))
            )
        )
    )
}

fn on_exit_menu_click<'a>(effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {
        effect_queue.borrow_mut().push_back(MenuEffect::PopMenu);
    }
}

fn on_exit_game_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}

fn on_open_to_lan_click<'a>(internal_server: &'a mut Option<InternalServer>) -> impl FnOnce(&GuiGlobalContext) + 'a {
    move |_| {
        if let &mut Some(ref mut internal_server) = internal_server {
            if internal_server.bind_to_lan.is_none() {
                let bind_to = "0.0.0.0:35565";
                info!("binding to {}", bind_to);
                internal_server.bind_to_lan = Some(internal_server.server.open_to_network(bind_to));
            } else {
                error!("already bound to lan");
            }
        } else {
            error!("cannot open to LAN because not the host");
        }
    }
}

fn on_options_click<'a>(effect_queue: &'a MenuEffectQueue) -> impl FnOnce(&GuiGlobalContext) + 'a {
    |_| {
        effect_queue.borrow_mut().push_back(MenuEffect::PushMenu(Menu::Settings));
    }
}
