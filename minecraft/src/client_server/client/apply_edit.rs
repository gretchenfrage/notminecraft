
use crate::client_server::message::{Edit, edit};
use crate::block_update_queue::BlockUpdateQueue;
use chunk_data::*;
use vek::*;


pub fn apply_edit(
    edit: Edit,
    cc: Vec3<i64>,
    ci: usize,
    getter: &Getter,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    block_updates: &mut BlockUpdateQueue,
) -> Edit {
    match edit {
        Edit::SetTileBlock(edit::SetTileBlock {
            lti,
            bid,
        }) => {
            let (old_bid, old_meta) = tile_blocks
                .get(cc, ci)
                .replace(lti, BlockId::new(bid), ());
            old_meta.cast::<()>();
            let gtc = cc_ltc_to_gtc(cc, lti_to_ltc(lti));
            block_updates.enqueue(gtc, getter);
            for face in FACES {
                block_updates.enqueue(gtc + face.to_vec(), getter);
            }
            edit::SetTileBlock {
                lti: lti,
                bid: old_bid,
            }.into()
        }
    }
}
