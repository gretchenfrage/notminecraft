//! Save "file" handling (actually a rocksdb database).

mod entry;

use crate::game_data::GameData;
use get_assets::DataDir;
use binschema::*;
use std::sync::Arc;
use anyhow::*;
use rocksdb::{
    DB,
    Options,
    WriteBatch,
};


pub use entry::{
    ReadKey,
    WriteEntry,
    read_key,
};


const SAVES_SUBDIR: &'static str = "saves";


/// Handle to an open save file. Operations are generally blocking.
///
/// Clone-shareable. Methods may take `&mut` merely as an optimization to reuse
/// allocations across operations within a handle, and thus do not actually imply
/// mutual exclusion of state changes beyond effects on operation performance.
pub struct SaveFile {
    shared: Arc<Shared>,
    coder_state_alloc: Option<CoderStateAlloc>,
    buf1: Vec<u8>,
    buf2: Vec<u8>,
}

// inner shared state
struct Shared {
    db: DB,
    key_schema: Schema,
    val_schemas: Vec<Schema>,
    game: Arc<GameData>,
}

impl SaveFile {
    /// Attempt to open existing save file, or create one if none exists, within
    /// data dir. This is a blocking operation.
    pub fn open(name: &str, data_dir: &DataDir, game: &Arc<GameData>) -> Result<Self> {
        // TODO: some sort of lease file
        
        // attempt to check whether database already exists
        let path = data_dir.subdir(SAVES_SUBDIR).join(name);
        let pre_existent = path.join("IDENTITY").try_exists()?;

        trace!(?pre_existent, "opening database");

        // open database, creating if doesn't yet exist
        let mut options = Options::default();
        options.create_if_missing(!pre_existent);
        let db = DB::open(&options, &path)?;

        // initialize or validate schema
        let my_schema_definition = entry::key_types(game);

        const SCHEMA_DEFINITION_KEY: &[u8] = &[0];

        let schema_definition_schema = schema_definition_schema();

        let mut coder_state_alloc = CoderStateAlloc::new();
        let mut buf = Vec::new();

        if pre_existent {
            // read saved schema definition
            let saved_schema_definition_bytes = db
                .get_pinned(SCHEMA_DEFINITION_KEY)?
                .ok_or_else(|| anyhow!(
                    "pre existent save file database is missing saved schema definition"
                ))?;

            // validate magic bytes
            ensure!(
                saved_schema_definition_bytes.len() >= 8,
                "pre existent save file database saved schema definition shorter than expected number of magic bytes",
            );
            ensure!(
                &saved_schema_definition_bytes[0..4] == &SAVE_FILE_MAGIC_BYTES,
                "pre existent save file database saved schema definition save file magic bytes wrong",
            );
            ensure!(
                &saved_schema_definition_bytes[4..8] == &Schema::schema_schema_magic_bytes(),
                "pre existent save file database saved schema definition schema schema magic bytes wrong",
            );

            // decode saved schema
            let mut coder_state = CoderState::new(&schema_definition_schema, coder_state_alloc, None);
            let saved_schema_definition = decode_schema_definition(
                &mut Decoder::new(
                    &mut coder_state,
                    &mut &saved_schema_definition_bytes[8..],
                )
            )
                .context("pre existent save file database saved schema definition failed to decode")?;

            // validate saved schema
            ensure!(
                saved_schema_definition == my_schema_definition,
                "pre existent save file database saved schema definition does not match expected schema definition\n\
                saved:\n{}\nexpected:\n{}",
                pretty_fmt_schema_definition(&saved_schema_definition),
                pretty_fmt_schema_definition(&my_schema_definition),
            );

            // reset coder state
            debug_assert!(coder_state.is_finished());
            coder_state_alloc = coder_state.into_alloc();
        } else {
            // encode schema, including magic bytes
            buf.extend(&SAVE_FILE_MAGIC_BYTES);
            buf.extend(&Schema::schema_schema_magic_bytes());
            let mut coder_state = CoderState::new(&schema_definition_schema, coder_state_alloc, None);
            encode_schema_definition(&my_schema_definition, &mut Encoder::new(&mut coder_state, &mut buf))?;

            // save to database
            db.put(SCHEMA_DEFINITION_KEY, &buf)?;

            // reset coder state
            debug_assert!(coder_state.is_finished());
            coder_state_alloc = coder_state.into_alloc();
        }

        info!(
            ?pre_existent,
            "successfully opened save file database with schema:\n{}\n",
            pretty_fmt_schema_definition(&my_schema_definition),
        );

        // build schema types for later use
        let mut key_schema_variants = Vec::new();
        let mut val_schemas = Vec::new();

        key_schema_variants.push(EnumSchemaVariant {
            name: "schema_definition".into(),
            inner: schema!(unit),
        }); 

        for (name, key_schema, val_schema) in my_schema_definition {
            key_schema_variants.push(EnumSchemaVariant {
                name,
                inner: key_schema,
            });
            val_schemas.push(val_schema);
        }

        // done
        Ok(SaveFile {
            shared: Arc::new(Shared {
                db,
                key_schema: Schema::Enum(key_schema_variants),
                val_schemas,
                game: Arc::clone(game),
            }),
            coder_state_alloc: Some(coder_state_alloc),
            buf1: buf,
            buf2: Vec::new(),
        })
    }

    /// Attempt to read the value at a given key. This is a blocking operation.
    pub fn read<R: ReadKey>(&mut self, key: R) -> Result<Option<R::Val>> {
        // encode key into buf1
        self.buf1.clear();
        let mut coder_state = CoderState::new(
            &self.shared.key_schema,
            self.coder_state_alloc.take().unwrap_or_default(),
            None,
        );
        key.encode_key(
            &mut Encoder::new(&mut coder_state, &mut self.buf1),
            &self.shared.game,
        )?;
        coder_state.is_finished_or_err()?;

        // get from database, short-circuit if None
        let val_bytes = match self.shared.db.get_pinned(&self.buf1)? {
            Some(val) => val,
            None => {
                self.coder_state_alloc = Some(coder_state.into_alloc());
                return Ok(None)
            }
        };

        // decode val
        let mut coder_state = CoderState::new(
            &self.shared.val_schemas[R::key_type_index()],
            coder_state.into_alloc(),
            None,
        );
        let val = R::decode_val(
            &mut Decoder::new(&mut coder_state, &mut &*val_bytes),
            &self.shared.game,
        )?;
        coder_state.is_finished_or_err()?;

        // done
        self.coder_state_alloc = Some(coder_state.into_alloc());
        Ok(Some(val))
    }

    /// Attempt to atomically make a batch of writes. This is a blocking operation.
    pub fn write(&mut self, writes: impl IntoIterator<Item=WriteEntry>) -> Result<()> {
        // prepare write batch
        let mut coder_state_alloc = self.coder_state_alloc.take().unwrap_or_default();
        let mut batch = WriteBatch::default();
        for write in writes {
            // encode key into buf1
            self.buf1.clear();
            let mut coder_state = CoderState::new(
                &self.shared.key_schema,
                coder_state_alloc,
                None,
            );
            write.encode_key(
                &mut Encoder::new(&mut coder_state, &mut self.buf1),
                &self.shared.game,
            )?;
            coder_state.is_finished_or_err()?;

            // encode val into buf2
            self.buf2.clear();
            let mut coder_state = CoderState::new(
                &self.shared.val_schemas[write.key_type_index()],
                coder_state.into_alloc(),
                None,
            );
            write.encode_val(
                &mut Encoder::new(&mut coder_state, &mut self.buf2),
                &self.shared.game,
            )?;
            coder_state.is_finished_or_err()?;

            // write key/value pair to write batch
            batch.put(&self.buf1, &self.buf2);

            // reset coder state alloc for next loop
            coder_state_alloc = coder_state.into_alloc();
        }

        // write to database
        self.shared.db.write(batch)?;

        // done
        self.coder_state_alloc = Some(coder_state_alloc);
        Ok(())
    }

    // TODO: deletions (add when needed)
}

impl Clone for SaveFile {
    fn clone(&self) -> Self {
        SaveFile {
            shared: Arc::clone(&self.shared),
            coder_state_alloc: None,
            buf1: Vec::new(),
            buf2: Vec::new(),
        }
    }
}


// ==== schema validation helper stuff ====


type SchemaDefinition = Vec<(String, Schema, Schema)>;

fn pretty_fmt_schema_definition(definition: &SchemaDefinition) -> String {
    use std::fmt::Write;

    let mut buf = String::new();
    for (i, &(ref name, ref key_schema, ref val_schema)) in definition.iter().enumerate() {
        write!(&mut buf, "key type {} (name = {:?})\n", i, name).unwrap();
        buf.push_str("key schema:\n");
        buf.push_str(&key_schema.pretty_fmt());
        buf.push_str("\nval schema:\n");
        buf.push_str(&val_schema.pretty_fmt());
        if i + 1 < definition.len() {
            buf.push('\n');
        }
    }
    buf
}

// magic bytes should be changed if schema definition schema, or other meta-level
// things about how the save file works, changes
const SAVE_FILE_MAGIC_BYTES: [u8; 4] = [0x2c, 0xbf, 0x35, 0x45];

fn schema_definition_schema() -> Schema {
    schema!(
        seq(varlen)(struct {
            (name: str),
            (key_schema: %Schema::schema_schema()),
            (val_schema: %Schema::schema_schema()),
        })
    )
}

fn encode_schema_definition(definition: &SchemaDefinition, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
    encoder.begin_var_len_seq(definition.len())?;
    for &(ref name, ref key_schema, ref val_schema) in definition {
        encoder.begin_seq_elem()?;
        encoder.begin_struct()?;
        encoder.begin_struct_field("name")?;
        encoder.encode_str(name)?;
        encoder.begin_struct_field("key_schema")?;
        key_schema.encode_schema(encoder)?;
        encoder.begin_struct_field("val_schema")?;
        val_schema.encode_schema(encoder)?;
        encoder.finish_struct()?;
    }
    Ok(encoder.finish_seq()?)
}

fn decode_schema_definition(decoder: &mut Decoder<&[u8]>) -> Result<SchemaDefinition> {
    let mut definition = Vec::new();
    for _ in 0..decoder.begin_var_len_seq()? {
        decoder.begin_seq_elem()?;
        decoder.begin_struct()?;
        definition.push((
            {
                decoder.begin_struct_field("name")?;
                decoder.decode_str()?
            },
            {
                decoder.begin_struct_field("key_schema")?;
                Schema::decode_schema(decoder)?
            },
            {
                decoder.begin_struct_field("val_schema")?;
                Schema::decode_schema(decoder)?
            },
        ));
        decoder.finish_struct()?;
    }
    decoder.finish_seq()?;
    Ok(definition)
}
