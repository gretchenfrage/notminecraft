
use super::{
    Server,
    OnReceived,
    OpenGameMenu,
    per_connection::*,
};
use crate::message::*;
use anyhow::*;


impl OnReceived<up::OpenGameMenu> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::OpenGameMenu, ck: ClientConnKey) -> Result<()> {
        let up::OpenGameMenu { menu } = msg;
        
        let valid = match &menu {
            &GameMenu::Inventory => true,
            &GameMenu::Chest { gtc: _ } => true, // TODO validation logic
        };
        let open_menu_msg_idx = self.last_processed[ck].num;
        if !valid {
            self.connections[ck].send(down::CloseGameMenu { open_menu_msg_idx });
        }
        self.open_game_menu[ck] = Some(OpenGameMenu {
            menu,
            open_menu_msg_idx,
            valid,
        });

        Ok(())
    }
}

impl OnReceived<up::CloseGameMenu> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::CloseGameMenu, ck: ClientConnKey) -> Result<()> {
        let up::CloseGameMenu {} = msg;

        self.open_game_menu[ck] = None;

        Ok(())
    }
}

impl OnReceived<up::GameMenuAction> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::GameMenuAction, ck: ClientConnKey) -> Result<()> {
        let up::GameMenuAction { action } = msg;
        match action {
            GameMenuAction::TransferItems {
                from,
                to,
                amount,
            } => {

            }
        }
        Ok(())
    }
}
