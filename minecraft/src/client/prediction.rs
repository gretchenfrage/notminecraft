
use super::{
    connection::Connection,
    apply_edit::apply_edit,
};
use crate::{
    message::*,
    block_update_queue::BlockUpdateQueue,
    util::sparse_vec::SparseVec,
};
use chunk_data::*;
use std::collections::VecDeque;
use vek::*;


#[derive(Debug)]
pub struct PredictionManager {
    // as predictions are made, they're pushed to the back
    predictions: VecDeque<Option<Prediction>>,
    // as serversides are received, they're pushed to the back
    serversides: VecDeque<Option<Serverside>>,
    // true for tiles iff there's an active prediction for that tile
    tile_has_prediction: PerChunk<PerTileBool>,
    
    // between operations, is false for all tiles
    tile_dont_stop_prediction: PerChunk<PerTileBool>,
    // between operations, is empty
    dont_stop_prediction_tiles: Vec<TileKey>,
}

#[derive(Debug)]
struct Prediction {
    reverser: Edit,
    ci: usize,
    up_msg_idx: u64,
}

#[derive(Debug)]
struct Serverside {
    edit: Edit,
    ci: usize,
}

impl PredictionManager {
    pub fn new() -> Self {
        PredictionManager {
            predictions: VecDeque::new(),
            serversides: VecDeque::new(),
            tile_has_prediction: PerChunk::new(),

            tile_dont_stop_prediction: PerChunk::new(),
            dont_stop_prediction_tiles: Vec::new(),
        }
    }

    pub fn add_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.tile_has_prediction.add(cc, ci, PerTileBool::new());
        self.tile_dont_stop_prediction.add(cc, ci, PerTileBool::new());
    }

    pub fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.tile_has_prediction.remove(cc, ci);
        self.tile_dont_stop_prediction.remove(cc, ci);

        for opt_prediction in &mut self.predictions {
            if opt_prediction.as_ref()
                .map(|prediction| prediction.ci == ci)
                .unwrap_or(false)
            {
                *opt_prediction = None;
            }
        }
        for opt_serverside in &mut self.serversides {
            if opt_serverside.as_ref()
                .map(|serverside| serverside.ci == ci)
                .unwrap_or(false)
            {
                *opt_serverside = None;
            }
        }
    }

    pub fn make_prediction(
        &mut self,
        edit: Edit,
        cc: Vec3<i64>, // TODO: it's a bit odd to be doing both of these like this
        ci: usize,
        getter: &Getter,
        connection: &Connection,
        tile_blocks: &mut PerChunk<ChunkBlocks>,
        block_updates: &mut BlockUpdateQueue,
    ) {
        let lti = edit_lti(&edit);
        trace!(?cc, ?ci, ?edit, "making prediction");
        let reverser = apply_edit(
            edit,
            cc,
            ci,
            getter,
            tile_blocks,
            block_updates,
        );
        self.predictions.push_back(Some(Prediction {
            reverser,
            ci,
            up_msg_idx: connection.up_msg_idx(),
        }));
        self.tile_has_prediction.get_mut(cc, ci).set(lti, true);
    }

    pub fn process_apply_edit_msg(
        &mut self,
        msg: down::ApplyEdit,
        chunks: &LoadedChunks,
        ci_reverse_lookup: &SparseVec<Vec3<i64>>,
        tile_blocks: &mut PerChunk<ChunkBlocks>,
        block_updates: &mut BlockUpdateQueue,
    ) {
        let down::ApplyEdit { ack, ci, edit } = msg;

        if let Some(last_processed) = ack {
            self.process_ack(
                last_processed,
                chunks,
                ci_reverse_lookup,
                tile_blocks,
                block_updates,
            );
        }

        let cc = ci_reverse_lookup[ci];
        let lti = edit_lti(&edit);
        if self.tile_has_prediction.get_mut(cc, ci).get(lti) {
            trace!(?cc, ?ci, edit=?edit, "stashing serverside");
            self.serversides.push_back(Some(Serverside {
                edit,
                ci,
            }));
        } else {
            trace!(?cc, ?ci, edit=?edit, "applying server edit");
            let getter = chunks.getter_pre_cached(cc, ci);
            apply_edit(
                edit,
                cc,
                ci,
                &getter,
                tile_blocks,
                block_updates,
            );
        }
    }

    pub fn process_ack(
        &mut self,
        last_processed: u64,
        chunks: &LoadedChunks,
        ci_reverse_lookup: &SparseVec<Vec3<i64>>,
        tile_blocks: &mut PerChunk<ChunkBlocks>,
        block_updates: &mut BlockUpdateQueue,
    ) {        
        // sweep through predictions backwards, from most recently predicted to least
        for opt_prediction in self.predictions.iter_mut().rev() {
            if let &mut Some(ref prediction) = opt_prediction {
                let ci = prediction.ci;
                let cc = ci_reverse_lookup[ci];
                let lti = edit_lti(&prediction.reverser);

                // if the prediction is a prediction of the consequences of sent messages
                // which we haven't received acknowledgement of the server processing
                if prediction.up_msg_idx > last_processed {
                    // make sure we don't stop those predictions

                    if !self.tile_dont_stop_prediction.get_mut(cc, ci).get(lti) {
                        self.tile_dont_stop_prediction.get_mut(cc, ci).set(lti, true);
                        self.dont_stop_prediction_tiles.push(TileKey { cc, ci, lti });
                    }

                    continue;
                } else if self.tile_dont_stop_prediction.get_mut(cc, ci).get(lti) {
                    // if this prediction is buried under predictions for which the above
                    // applies, make sure not to roll it back in that case either
                    continue;
                }

                trace!(?cc, ?ci, reverser=?prediction.reverser, "rolling back prediction");

                // but if neither of the above apply here, we can roll it back
                // (note: this sets the item in the queue to none)
                let prediction = opt_prediction.take().unwrap();
                let getter = chunks.getter_pre_cached(cc, ci);
                apply_edit(
                    prediction.reverser,
                    cc,
                    ci,
                    &getter,
                    tile_blocks,
                    block_updates,
                );

                // and if we're rolling back this prediction for the tile, that means we'll
                // eventually in this loop roll back all the predictions on this tile, so
                // we can just go ahead and mark that tile as not having predictions any more
                self.tile_has_prediction.get_mut(cc, ci).set(lti, false);
            }
        }

        // now, sweep through serversides forwards, from least recently received serversides
        // to most, and apply any serversides that now have the predictions that were parallel
        // to them rolled back
        for opt_prediction in self.serversides.iter_mut() {
            if let &mut Some(ref serverside) = opt_prediction {
                let ci = serverside.ci;
                let cc = ci_reverse_lookup[ci];
                let lti = edit_lti(&serverside.edit);

                if !self.tile_has_prediction.get(cc, ci).get(lti) {
                    trace!(?cc, ?ci, edit=?serverside.edit, "applying serverside");

                    // (note: this sets the item in the queue to none)
                    let Serverside { ci: _, edit } = opt_prediction.take().unwrap();
                    let getter = chunks.getter_pre_cached(cc, ci);
                    apply_edit(
                        edit,
                        cc,
                        ci,
                        &getter,
                        tile_blocks,
                        block_updates,
                    );
                }
            }
        }

        // finally, do some cleanup

        // reset the "dont stop prediction" data structures
        for tile in self.dont_stop_prediction_tiles.drain(..) {
            tile.set(&mut self.tile_dont_stop_prediction, false);
        }

        // garbage collect the queues a bit
        trim_queue(&mut self.predictions);
        trim_queue(&mut self.serversides);
    }
}

// when edit types are introduced that don't really operate on the level of
// a tile, this can be generalized into some sort of "edit scope"
//
// also, worth noting that we're assuming here that a reverser will apply to
// the same lti (or scope, if we generalize) as its original edit. which seems
// fine.
fn edit_lti(edit: &Edit) -> u16 {
    match edit {
        &Edit::SetTileBlock(edit::SetTileBlock { lti, .. }) => lti,
    }
}

fn trim_queue<T>(queue: &mut VecDeque<Option<T>>) {
    while matches!(queue.back(), Some(&None)) {
        queue.pop_back();
    }
    while matches!(queue.front(), Some(&None)) {
        queue.pop_front();
    }
}
