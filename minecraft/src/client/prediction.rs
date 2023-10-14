
use super::{
    connection::Connection,
    apply_edit::*,
};
use crate::message::*;
use chunk_data::*;
use std::collections::VecDeque;
use vek::*;


/// Manages client side prediction.
#[derive(Debug)]
pub struct PredictionManager {
    // as predictions are made, they're pushed to the back
    predictions: VecDeque<Option<Prediction>>,
    // as serversides are received, they're pushed to the back
    serversides: VecDeque<Option<Edit>>,

    scope_state: ScopeState,
}

/// An active client side prediction.
#[derive(Debug)]
struct Prediction {
    /// Edit to undo the prediction.
    reverser: Edit,
    /// This is a prediction of the consequences of the processing of messages
    /// we sent up to that up msg idx.
    up_msg_idx: u64,
}

/// Prediction manager state for each scope currently in the world.
#[derive(Debug)]
struct ScopeState {
    // true for scope iff there's an active prediction for that scope
    has_prediction_tile: PerChunk<PerTileBool>,
    has_prediction_inventory_slot: [bool; 36],
    
    // between operations, is false for all scope
    dont_stop_prediction_tile: PerChunk<PerTileBool>,
    dont_stop_prediction_inventory_slot: [bool; 36],

    // between operations, is empty
    dont_stop_prediction_list_tile: Vec<TileKey>,
    dont_stop_prediction_list_inventory_slot: Vec<usize>,
}

impl PredictionManager {
    pub fn new() -> Self {
        PredictionManager {
            predictions: VecDeque::new(),
            serversides: VecDeque::new(),

            scope_state: ScopeState {
                has_prediction_tile: PerChunk::new(),
                has_prediction_inventory_slot: [false; 36],

                dont_stop_prediction_tile: PerChunk::new(),
                dont_stop_prediction_inventory_slot: [false; 36],

                dont_stop_prediction_list_tile: Vec::new(),
                dont_stop_prediction_list_inventory_slot: Vec::new(),
            }
        }
    }

    pub fn add_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.scope_state.has_prediction_tile.add(cc, ci, PerTileBool::new());
        self.scope_state.dont_stop_prediction_tile.add(cc, ci, PerTileBool::new());
    }

    pub fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.scope_state.has_prediction_tile.remove(cc, ci);
        self.scope_state.dont_stop_prediction_tile.remove(cc, ci);

        for opt_prediction in &mut self.predictions {
            if opt_prediction.as_ref()
                .map(|prediction| match &prediction.reverser {
                    &Edit::Tile(ref edit) => edit.ci == ci,
                    _ => false,
                })
                .unwrap_or(false)
            {
                *opt_prediction = None;
            }
        }
        for opt_serverside in &mut self.serversides {
            if opt_serverside.as_ref()
                .map(|serverside| match serverside {
                    &Edit::Tile(ref edit) => edit.ci == ci,
                    _ => false,
                })
                .unwrap_or(false)
            {
                *opt_serverside = None;
            }
        }
    }

    pub fn make_prediction(
        &mut self,
        edit: Edit,
        world: &mut EditWorld,
        connection: &Connection,
    ) {
        self.scope_state.do_edit_op(
            edit,
            Op {
                connection,
                predictions: &mut self.predictions,
            },
            world,
        );

        struct Op<'a> {
            connection: &'a Connection,
            predictions: &'a mut VecDeque<Option<Prediction>>,
        }

        impl<'a> EditOp for Op<'a> {
            fn call<E: EditVariant>(self, edit: E, ctx: EditOpCtx<E::Key>) {
                trace!(?edit, "making prediction");

                let reverser = edit.apply(ctx.world);
                *ctx.has_prediction = true;
                self.predictions.push_back(Some(Prediction {
                    reverser,
                    up_msg_idx: self.connection.up_msg_idx(),
                }));
            }
        }
    }

    pub fn process_apply_edit_msg(
        &mut self,
        msg: down::ApplyEdit,
        world: &mut EditWorld,
    ) {
        let down::ApplyEdit { ack, edit } = msg;

        if let Some(last_processed) = ack {
            self.process_ack(
                last_processed,
                world,
            );
        }

        self.scope_state.do_edit_op(
            edit,
            Op {
                serversides: &mut self.serversides,
            },
            world,
        );

        struct Op<'a> {
            serversides: &'a mut VecDeque<Option<Edit>>,
        }

        impl<'a> EditOp for Op<'a> {
            fn call<E: EditVariant>(self, edit: E, ctx: EditOpCtx<E::Key>) {
                // if has_prediction, edit is blocked, else, apply it now
                if *ctx.has_prediction {
                    self.serversides.push_back(Some(edit.into()));
                } else {
                    edit.apply(ctx.world);
                }
            }
        }
    }

    pub fn process_ack(
        &mut self,
        last_processed: u64,
        world: &mut EditWorld
    ) {        
        // sweep through predictions backwards, from most recently predicted to least
        for opt_prediction in self.predictions.iter_mut().rev() {
            // for ownership reasons, we always take it from the queue, but
            // then put it back if we dont actually want it to be taken
            if let Some(Prediction { reverser, up_msg_idx }) = opt_prediction.take() {
                self.scope_state.do_edit_op(
                    reverser,
                    Op {
                        opt_prediction,
                        up_msg_idx,
                        last_processed,
                    },
                    world,
                );

                struct Op<'a> {
                    opt_prediction: &'a mut Option<Prediction>,
                    up_msg_idx: u64,
                    last_processed: u64,
                }

                impl<'a> EditOp for Op<'a> {
                    fn call<E: EditVariant>(self, edit: E, ctx: EditOpCtx<E::Key>) {
                        // if the prediction is a prediction of the consequences of sent messages
                        // which we haven't received acknowledgement of the server processing
                        if self.up_msg_idx > self.last_processed {
                            // make sure we don't stop those predictions

                            if !*ctx.dont_stop_prediction {
                                *ctx.dont_stop_prediction = true;
                                ctx.dont_stop_prediction_list.push(edit.key(ctx.world));
                            }

                            *self.opt_prediction = Some(Prediction { reverser: edit.into(), up_msg_idx: self.up_msg_idx });
                            return;
                        } else if *ctx.dont_stop_prediction {
                            // if this prediction is buried under predictions for which the above
                            // applies, make sure not to roll it back in that case either
                            *self.opt_prediction = Some(Prediction { reverser: edit.into(), up_msg_idx: self.up_msg_idx });
                            return;
                        }

                        trace!(reverser=?edit, "rolling back prediction");

                        // but if neither of the above apply here, we can roll it back
                        edit.apply(ctx.world);

                        // and if we're rolling back this prediction for the tile, that means we'll
                        // eventually in this loop roll back all the predictions on this tile, so
                        // we can just go ahead and mark that tile as not having predictions any more
                        *ctx.has_prediction = false;
                    }
                }
            }
        }

        // now, sweep through serversides forwards, from least recently received serversides
        // to most, and apply any serversides that now have the predictions that were parallel
        // to them rolled back
        for opt_serverside in self.serversides.iter_mut() {
            // for ownership reasons, we always take it from the queue, but
            // then put it back if we dont actually want it to be taken
            if let Some(serverside) = opt_serverside.take() {
                self.scope_state.do_edit_op(
                    serverside,
                    Op {
                        opt_serverside,
                    },
                    world,
                );

                struct Op<'a> {
                    opt_serverside: &'a mut Option<Edit>,
                }

                impl<'a> EditOp for Op<'a> {
                    fn call<E: EditVariant>(self, edit: E, ctx: EditOpCtx<E::Key>) {
                        if !*ctx.has_prediction {
                            trace!(?edit, "applying serverside");
                            edit.apply(ctx.world);
                        } else {
                            *self.opt_serverside = Some(edit.into());
                        }
                    }
                }
            }
        }

        // finally, do some cleanup

        // reset the "dont stop prediction" data structures
        for tile in self.scope_state.dont_stop_prediction_list_tile.drain(..) {
            tile.set(&mut self.scope_state.dont_stop_prediction_tile, false);
        }
        for slot_idx in self.scope_state.dont_stop_prediction_list_inventory_slot.drain(..) {
            self.scope_state.dont_stop_prediction_inventory_slot[slot_idx] = false;
        }

        // garbage collect the queues a bit
        trim_queue(&mut self.predictions);
        trim_queue(&mut self.serversides);
    }
}

impl ScopeState {
    /// Do an abstracted operation on an edit.
    fn do_edit_op(
        &mut self,
        edit: Edit,
        op: impl EditOp,
        world: &mut EditWorld,
    ) {
        match edit {
            Edit::Tile(edit) => {
                let tile = edit.key(world);
                let has_prediction_1 = tile.get(&self.has_prediction_tile);
                let dont_stop_prediction_1 = tile.get(&self.dont_stop_prediction_tile);
                let mut has_prediction_2 = has_prediction_1;
                let mut dont_stop_prediction_2 = dont_stop_prediction_1;
                op.call(
                    edit,
                    EditOpCtx {
                        world,
                        has_prediction: &mut has_prediction_2,
                        dont_stop_prediction: &mut dont_stop_prediction_2,
                        dont_stop_prediction_list: &mut self.dont_stop_prediction_list_tile,
                    }
                );
                if has_prediction_2 != has_prediction_1 {
                    tile.set(&mut self.has_prediction_tile, has_prediction_2);
                }
                if dont_stop_prediction_2 != dont_stop_prediction_1 {
                    tile.set(&mut self.dont_stop_prediction_tile, dont_stop_prediction_2);
                }
            }
            Edit::InventorySlot(edit) => {
                let slot_idx = edit.slot_idx;
                op.call(
                    edit,
                    EditOpCtx {
                        world,
                        has_prediction: &mut self.has_prediction_inventory_slot[slot_idx],
                        dont_stop_prediction: &mut self.dont_stop_prediction_inventory_slot[slot_idx],
                        dont_stop_prediction_list: &mut self.dont_stop_prediction_list_inventory_slot,
                    }
                );
            }
        }
    }
}

/// An abstracted operation on an edit. Would just use closures but is generic.
trait EditOp {
    fn call<E: EditVariant>(self, edit: E, ctx: EditOpCtx<E::Key>);
}

#[derive(Debug)]
struct EditOpCtx<'a, 'b, 'c, K> {
    world: &'b mut EditWorld<'c>,

    has_prediction: &'a mut bool,
    dont_stop_prediction: &'a mut bool,
    dont_stop_prediction_list: &'a mut Vec<K>,
}

fn trim_queue<T>(queue: &mut VecDeque<Option<T>>) {
    while matches!(queue.back(), Some(&None)) {
        queue.pop_back();
    }
    while matches!(queue.front(), Some(&None)) {
        queue.pop_front();
    }
}
