
use crate::{
    game_data::GameData,
    util_array::ArrayBuilder,
    item::{
        ItemStack,
        RawItemId,
    },
};
use binschema::{*, error::*};
use chunk_data::*;
use std::{
    collections::*,
    sync::Arc,
    iter,
    hash::Hash,
};
use vek::*;


pub use game_binschema_derive::GameBinschema;


/// Types which know their schema and how to transcode themselves with
/// binschema, given `GameData`. The `game_binschema_derive` package
/// in the workspace provides a derive proc macro for this :).
pub trait GameBinschema {
    fn schema(game: &Arc<GameData>) -> Schema;
    
    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()>;

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self>
    where
        Self: Sized;
}

macro_rules! scalar_game_binschema {
    ($t:ident, $encode:ident, $decode:ident)=>{
        impl GameBinschema for $t {
            fn schema(_: &Arc<GameData>) -> Schema {
                schema!($t)
            }

            fn encode(&self, encoder: &mut Encoder<Vec<u8>>, _: &Arc<GameData>) -> Result<()> {
                encoder.$encode(*self)
            }

            fn decode(decoder: &mut Decoder<&[u8]>, _: &Arc<GameData>) -> Result<Self> {
                decoder.$decode()
            }
        }
    };
}

scalar_game_binschema!(u8, encode_u8, decode_u8);
scalar_game_binschema!(u16, encode_u16, decode_u16);
scalar_game_binschema!(u32, encode_u32, decode_u32);
scalar_game_binschema!(u64, encode_u64, decode_u64);
scalar_game_binschema!(u128, encode_u128, decode_u128);
scalar_game_binschema!(i8, encode_i8, decode_i8);
scalar_game_binschema!(i16, encode_i16, decode_i16);
scalar_game_binschema!(i32, encode_i32, decode_i32);
scalar_game_binschema!(i64, encode_i64, decode_i64);
scalar_game_binschema!(i128, encode_i128, decode_i128);
scalar_game_binschema!(f32, encode_f32, decode_f32);
scalar_game_binschema!(f64, encode_f64, decode_f64);
scalar_game_binschema!(char, encode_char, decode_char);
scalar_game_binschema!(bool, encode_bool, decode_bool);

macro_rules! size_game_binschema {
    ($size:ident, $fixed:ident, $encode:ident, $decode:ident)=>{
        impl GameBinschema for $size {
            fn schema(_: &Arc<GameData>) -> Schema {
                schema!($fixed)
            }

            fn encode(&self, encoder: &mut Encoder<Vec<u8>>, _: &Arc<GameData>) -> Result<()> {
                encoder.$encode(*self as $fixed)
            }

            fn decode(decoder: &mut Decoder<&[u8]>, _: &Arc<GameData>) -> Result<Self> {
                decoder.$decode()
                    .and_then(|n| $size::try_from(n)
                        .map_err(|e| Error::new(
                            ErrorKind::PlatformLimits,
                            e,
                            Some(decoder.coder_state()),
                        )))
            }
        }
    };
}

size_game_binschema!(usize, u64, encode_u64, decode_u64);
size_game_binschema!(isize, i64, encode_i64, decode_i64);

impl GameBinschema for String {
    fn schema(_: &Arc<GameData>) -> Schema {
        schema!(str)
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, _: &Arc<GameData>) -> Result<()> {
        encoder.encode_str(self)
    }

    fn decode(decoder: &mut Decoder<&[u8]>, _: &Arc<GameData>) -> Result<Self> {
        decoder.decode_str()
    }
}

impl GameBinschema for () {
    fn schema(_: &Arc<GameData>) -> Schema {
        schema!(unit)
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, _: &Arc<GameData>) -> Result<()> {
        encoder.encode_unit()
    }

    fn decode(decoder: &mut Decoder<&[u8]>, _: &Arc<GameData>) -> Result<Self> {
        decoder.decode_unit()
    }
}

impl<T: GameBinschema> GameBinschema for Option<T> {
    fn schema(game: &Arc<GameData>) -> Schema {
        schema!(option(%T::schema(game)))
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        if let &Some(ref inner) = self {
            encoder.begin_some()?;
            inner.encode(encoder, game)
        } else {
            encoder.encode_none()
        }
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        Ok(if decoder.begin_option()? {
            Some(T::decode(decoder, game)?)
        } else {
            None
        })
    }
}

macro_rules! seq_game_binschema {
    ($c:ident $($bounds:tt)*)=>{
        impl<T: GameBinschema $($bounds)*> GameBinschema for $c<T> {
            fn schema(game: &Arc<GameData>) -> Schema {
                schema!(seq(varlen)(%T::schema(game)))
            }

            fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                encoder.begin_var_len_seq(self.len())?;
                for elem in self {
                    encoder.begin_seq_elem()?;
                    elem.encode(encoder, game)?;
                }
                encoder.finish_seq()
            }

            fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
                let len = decoder.begin_var_len_seq()?;
                let collection = iter::from_fn(|| Some(
                    decoder.begin_seq_elem()
                        .and_then(|()| T::decode(decoder, game))
                ))
                    .take(len)
                    .collect::<Result<$c<T>>>()?;
                decoder.finish_seq()?;
                Ok(collection)
            }
        }
    };
}

seq_game_binschema!(Vec);
seq_game_binschema!(BinaryHeap + Ord);
seq_game_binschema!(BTreeSet + Ord);
seq_game_binschema!(HashSet + Hash + Eq);
seq_game_binschema!(LinkedList);
seq_game_binschema!(VecDeque);

macro_rules! map_game_binschema {
    ($c:ident $($bounds:tt)*)=>{
        impl<K: GameBinschema $($bounds)*, V: GameBinschema> GameBinschema for $c<K, V> {
            fn schema(game: &Arc<GameData>) -> Schema {
                schema!(seq(varlen)(tuple {
                    (%K::schema(game)),
                    (%V::schema(game)),
                }))
            }

            fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                encoder.begin_var_len_seq(self.len())?;
                for (key, val) in self {
                    encoder.begin_seq_elem()?;
                    encoder.begin_tuple()?;
                    encoder.begin_tuple_elem()?;
                    key.encode(encoder, game)?;
                    encoder.begin_tuple_elem()?;
                    val.encode(encoder, game)?;
                    encoder.finish_tuple()?;
                }
                encoder.finish_seq()
            }

            fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
                let len = decoder.begin_var_len_seq()?;
                let collection = iter::from_fn(|| Some(
                    decoder.begin_seq_elem()
                        .and_then(|()| decoder.begin_tuple())
                        .and_then(|()| decoder.begin_tuple_elem())
                        .and_then(|()| K::decode(decoder, game))
                        .and_then(|k| decoder.begin_tuple_elem()
                            .map(move |()| k))
                        .and_then(|k| V::decode(decoder, game)
                            .map(move |v| (k, v)))
                        .and_then(|kv| decoder.finish_tuple()
                            .map(move |()| kv))
                ))
                    .take(len)
                    .collect::<Result<$c<K, V>>>()?;
                decoder.finish_seq()?;
                Ok(collection)
            }
        }
    };
}

map_game_binschema!(BTreeMap + Ord);
map_game_binschema!(HashMap + Hash + Eq);

impl<T: GameBinschema, const N: usize> GameBinschema for [T; N] {
    fn schema(game: &Arc<GameData>) -> Schema {
        schema!(seq(N)(%T::schema(game)))
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        encoder.begin_fixed_len_seq(N)?;
        for elem in self {
            encoder.begin_seq_elem()?;
            elem.encode(encoder, game)?;
        }
        encoder.finish_seq()
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        let mut array = ArrayBuilder::new();
        decoder.begin_fixed_len_seq(N)?;
        for _ in 0..N {
            decoder.begin_seq_elem()?;
            array.push(T::decode(decoder, game)?);
        }
        decoder.finish_seq()?;
        Ok(array.build())
    }
}

macro_rules! tuples_game_binschema {
    (@inner $($v:ident $t:ident),*)=>{
        impl<$($t: GameBinschema),*> GameBinschema for ($($t,)*) {
            fn schema(game: &Arc<GameData>) -> Schema {
                schema!(tuple {$(
                    (%$t::schema(game)),
                )*})
            }

            fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                encoder.begin_tuple()?;
                let &(
                    $(ref $v,)*
                ) = self;
                $(
                    encoder.begin_tuple_elem()?;
                    $v.encode(encoder, game)?;
                )*
                encoder.finish_tuple()
            }

            fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
                decoder.begin_tuple()?;
                $(
                    decoder.begin_tuple_elem()?;
                    let $v = $t::decode(decoder, game)?;
                )*
                decoder.finish_tuple()?;
                Ok(($(
                    $v,
                )*))
            }
        }
    };
    ($v1:ident $t1:ident $(, $v:ident $t:ident)*)=>{
        tuples_game_binschema!(@inner $v1 $t1 $(, $v $t)*);
        tuples_game_binschema!($($v $t),*);
    };
    ()=>{};
}

tuples_game_binschema!(a A, b B, c C, d D, e E, f F, g G, h H, i I, j J, k K);

impl<T: GameBinschema> GameBinschema for Box<T> {
    fn schema(game: &Arc<GameData>) -> Schema {
        T::schema(game)
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        T::encode(&**self, encoder, game)
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        T::decode(decoder, game).map(Box::new)
    }
}
/*
impl<T: GameBinschema> GameBinschema for RefCell<T> {
    fn schema(game: &Arc<GameData>) -> Schema {
        T::schema(game)
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        T::encode(&*self.borrow(), encoder, game)
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        T::decode(decoder, game).map(RefCell::new)
    }
}
*/
macro_rules! vek_vec_game_binschema {
    ($n:expr, $v:ident {$( $f:ident ),*})=>{
        impl<T: GameBinschema> GameBinschema for $v<T> {
            fn schema(game: &Arc<GameData>) -> Schema {
                schema!(
                    seq($n)(%T::schema(game))
                )
            }

            fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                encoder.begin_fixed_len_seq($n)?;
                $(
                    encoder.begin_seq_elem()?;
                    self.$f.encode(encoder, game)?;
                )*
                encoder.finish_seq()
            }

            fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
                decoder.begin_fixed_len_seq($n)?;
                $(
                    decoder.begin_seq_elem()?;
                    let $f = T::decode(decoder, game)?;
                )*
                decoder.finish_seq()?;
                Ok($v {$( $f ),*})
            }
        }
    };
}

vek_vec_game_binschema!(2, Vec2 { x, y });
vek_vec_game_binschema!(3, Vec3 { x, y, z });
vek_vec_game_binschema!(4, Vec4 { x, y, z, w });
vek_vec_game_binschema!(2, Extent2 { w, h });
vek_vec_game_binschema!(3, Extent3 { w, h, d });
vek_vec_game_binschema!(3, Rgb { r, g, b });
vek_vec_game_binschema!(4, Rgba { r, g, b, a });

impl GameBinschema for RawBlockId {
    fn schema(game: &Arc<GameData>) -> Schema {
        Schema::Enum(game.blocks.iter()
            .map(|bid| EnumSchemaVariant {
                name: game.blocks_machine_name.get(bid).clone(),
                inner: schema!(unit),
            })
            .collect())
    }
    
    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        encoder.begin_enum(self.0 as usize, &game.blocks_machine_name.get(*self))?;
        encoder.encode_unit()
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        let bid = RawBlockId(decoder.begin_enum()? as u16);
        decoder.begin_enum_variant(&game.blocks_machine_name.get(bid))?;
        decoder.decode_unit()?;
        Ok(bid)
    }
}

impl GameBinschema for ErasedBidMeta {
    fn schema(game: &Arc<GameData>) -> Schema {
        Schema::Enum(game.blocks.iter()
            .map(|bid| EnumSchemaVariant {
                name: game.blocks_machine_name[bid].clone(),
                inner: game.blocks_meta_transcloner[bid].instance_schema(game),
            })
            .collect())
    }
    
    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        encoder.begin_enum(self.bid.0 as usize, &game.blocks_machine_name[self.bid])?;
        game.blocks_meta_transcloner[self.bid].encode_erased_block_meta(
            &self.meta,
            encoder,
            game,
        )
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        let bid = RawBlockId(decoder.begin_enum()? as u16);
        decoder.begin_enum_variant(&game.blocks_machine_name[bid])?;
        let meta = game.blocks_meta_transcloner[bid].decode_erased_block_meta(
            decoder,
            game,
        )?;
        Ok(ErasedBidMeta { bid, meta })
    }
}

impl GameBinschema for ChunkBlocks {
    fn schema(game: &Arc<GameData>) -> Schema {
        schema!(
            seq(NUM_LTIS)(%ErasedBidMeta::schema(game))
        )
    }
    
    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        encoder.begin_fixed_len_seq(NUM_LTIS)?;
        for lti in 0..=MAX_LTI {
            encoder.begin_seq_elem()?;
            let bid = self.get(lti);
            encoder.begin_enum(bid.0 as usize, &game.blocks_machine_name[bid])?;
            game.blocks_meta_transcloner[bid].encode_tile_block_meta(
                TileBlockRead { chunk: self, lti },
                encoder,
                game,
            )?;
        }
        encoder.finish_seq()
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        decoder.begin_fixed_len_seq(NUM_LTIS)?;
        let mut chunk = ChunkBlocks::new(&game.blocks);
        for lti in 0..=MAX_LTI {
            decoder.begin_seq_elem()?;
            let bid = RawBlockId(decoder.begin_enum()? as u16);
            decoder.begin_enum_variant(&game.blocks_machine_name[bid])?;
            game.blocks_meta_transcloner[bid].decode_tile_block_meta(
                bid,
                TileBlockWrite { chunk: &mut chunk, lti },
                decoder,
                game,
            )?;
        }
        decoder.finish_seq()?;
        Ok(chunk)
    }
}

impl GameBinschema for ItemStack {
    fn schema(game: &Arc<GameData>) -> Schema {
        schema!(
            struct {
                (item: %Schema::Enum(game.items.iter()
                    .map(|iid| EnumSchemaVariant {
                        name: game.items_machine_name[iid].clone(),
                        inner: game.items_meta_transcloner[iid].instance_schema(game),
                    })
                    .collect())),
                (count: u8),
                (damage: u16),
            }
        )
    }

    fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
        encoder.begin_struct()?;
        encoder.begin_struct_field("item")?;
        encoder.begin_enum(self.iid.0 as usize, &game.items_machine_name[self.iid])?;
        game.items_meta_transcloner[self.iid].encode_item_meta(
            &self.meta,
            encoder,
            game,
        )?;
        encoder.begin_struct_field("count")?;
        encoder.encode_u8(self.count.get())?;
        encoder.begin_struct_field("damage")?;
        encoder.encode_u16(self.damage)?;
        encoder.finish_struct()
    }

    fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        decoder.begin_struct()?;
        decoder.begin_struct_field("item")?;
        let iid = RawItemId(decoder.begin_enum()? as u16);
        decoder.begin_enum_variant(&game.items_machine_name[iid])?;
        let meta = game.items_meta_transcloner[iid].decode_item_meta(
            decoder,
            game,
        )?;
        let val = ItemStack {
            iid,
            meta,
            count: {
                decoder.begin_struct_field("count")?;
                decoder.decode_u8()?
                    .try_into()
                    .map_err(|_| Error::other("decoded ItemStack with count=0"))?
            },
            damage: {
                decoder.begin_struct_field("damage")?;
                decoder.decode_u16()?
            },
        };
        decoder.finish_struct()?;
        Ok(val)
    }
}
