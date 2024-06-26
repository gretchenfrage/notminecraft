
use crate::{
    server::ServerSyncCtx,
    message::*,
};
use vek::*;

pub const NUM_STEVES: usize = 10;
pub const STEVE_HEIGHT: f32 = 1.8;
pub const STEVE_WIDTH: f32 = 0.6;

#[derive(Debug, Clone, Default)]
pub struct Steve {
    pub pos: Vec3<f32>,
    pub vel: Vec3<f32>,
}

pub struct SyncWrite<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut [Steve; NUM_STEVES],
}

impl<'a> SyncWrite<'a> {
    pub fn new_manual(ctx: &'a ServerSyncCtx, state: &'a mut [Steve; NUM_STEVES]) -> Self {
        SyncWrite { ctx, state }
    }

    pub fn as_ref(&self) -> &[Steve; NUM_STEVES] {
        &self.state
    }

    pub fn get(&mut self, steve_idx: usize) -> SyncWriteSteve {
        SyncWriteSteve {
            ctx: self.ctx,
            state: &mut self.state[steve_idx],
            steve_idx,
        }
    }
}

pub struct SyncWriteSteve<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut Steve,
    steve_idx: usize,
}

impl<'a> SyncWriteSteve<'a> {
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteSteve<'a2> {
        SyncWriteSteve {
            ctx: &self.ctx,
            state: &mut self.state,
            steve_idx: self.steve_idx,
        }
    }

    pub fn as_ref(&self) -> &Steve {
        &self.state
    }

    pub fn set_pos_vel(&mut self, pos: Vec3<f32>, vel: Vec3<f32>) {
        for pk in self.ctx.conn_mgr.players().iter() {
            self.ctx.conn_mgr
                .send(pk, DownMsg::PreJoin(PreJoinDownMsg::SetStevePosVel {
                    steve_idx: self.steve_idx,
                    pos,
                    vel,
                }));
        }

        // TODO save file

        self.state.pos = pos;
        self.state.vel = vel;
    }
}
