//! Reading and writing the save file. See also the `save_content` module.
//!
//! The save file database essentially has 3 layers of abstraction:
//!
//! 1. Key/value store. We just need a generic key/value database to go here. Currently we use
//!    redb. We'd like write transactions to be supported, so that a partial save doesn't result
//!    in an inconsistent world state.
//! 2. Binschema integration. This involves storing the database's key/value schema within the
//!    database itself so that we don't accidentally misinterpret bytes stored in it as meaning
//!    something else.
//!
//!    We store the database's schema under a single special key consisting of a single 0 byte. The
//!    corresponding value is a concatenation of:
//!
//!    - A constant defined here, the "save file magic bytes", which should be changed if this
//!      binschema integration protocol is changed.
//!    - A constant defined in binschema, the "schema schema magic bytes", which should be changed
//!      if binschema's behavior or the schema of schemas are changed.
//!    - The binschema-encoded key/value schema definition, of the schema:
//!
//!      ```
//!      - seq (variable length):
//!          - struct
//!            field 0 (name = "name"):
//!                - str
//!            field 1 (name = "key_schema"):
//!                - %(schema schema)
//!            field 2 (name = "val_schema"):
//!                - %(schema schema)
//!      ```
//!
//!    That key/value schema definition defines the sequence of key types and their corresponding
//!    value types. The actual schema used when transcoding a key generally is an enum, wherein:
//!
//!    - Variant 0 is a "dummy variant" which is never initialized, so as to ensure that a database
//!      key begins with 0 iff it contains the database schema definition.
//!    - The rest of the variants are generated from the key types names and key schemas as in the
//!      key/val schema definition.
//!
//!    The values are simply the binschema-encoded value, encoded with the appropriate val schema
//!    for its corresponding key type.
//! 3. Our particular current schema. This is defined in structs and macros in the `save_content`
//!    module. The underlying layers allows this to be changed without causing corruption.

use crate::{
    server::save_content::*,
    game_binschema::*,
    game_data::GameData,
};
use get_assets::DataDir;
use binschema::*;
use std::sync::Arc;
use anyhow::*;
use redb::{
    Database,
    TableDefinition,
    ReadableTable,
};


const SAVES_SUBDIR: &'static str = "saves";
const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("save");

// magic bytes should be changed if schema definition schema, or other meta-level
// things about how the save file works, changes
const SAVE_FILE_MAGIC_BYTES: [u8; 4] = [0x2c, 0xbf, 0x35, 0x45];


/// Open handle for reading and writing a save file database.
///
/// Operations are blocking.
#[derive(Debug)]
pub struct SaveDb {
    shared: Arc<Shared>,
    coder_state_alloc: Option<CoderStateAlloc>,
    buf1: Vec<u8>,
    buf2: Vec<u8>,
}

// inner shared state
#[derive(Debug)]
struct Shared {
    db: Database,
    key_schema: Schema,
    val_schemas: Vec<Schema>,
    game: Arc<GameData>,
}

impl SaveDb {
    /// Open existing save file, or create one of the path is empty.
    pub fn open(name: &str, data_dir: &DataDir, game: &Arc<GameData>) -> Result<Self> {        
        // attempt to check whether database already exists
        let mut name = name.to_owned();
        name.push_str(".redb");
        let path = data_dir.subdir(SAVES_SUBDIR).join(name);
        let pre_existent = path.try_exists()?;

        trace!(?pre_existent, "opening database");

        // open database, creating if doesn't yet exist
        let db = Database::create(path)?;

        // initialize or validate schema
        let my_schema_definition = current_save_schema(game);

        const SCHEMA_DEFINITION_KEY: &[u8] = &[0];

        let schema_definition_schema = schema_definition_schema();

        let mut coder_state_alloc = CoderStateAlloc::new();
        let mut buf = Vec::new();

        if pre_existent {
            // read saved schema definition
            let txn = db.begin_read()?;
            let table = txn.open_table(TABLE)?;
            let saved_schema_definition_bytes = table
                .get(SCHEMA_DEFINITION_KEY)?
                .ok_or_else(|| anyhow!(
                    "pre existent save file database is missing saved schema definition"
                ))?;

            // validate magic bytes
            ensure!(
                saved_schema_definition_bytes.value().len() >= 8,
                "pre existent save file database saved schema definition shorter than expected number of magic bytes",
            );
            ensure!(
                &saved_schema_definition_bytes.value()[0..4] == &SAVE_FILE_MAGIC_BYTES,
                "pre existent save file database saved schema definition save file magic bytes wrong",
            );
            ensure!(
                &saved_schema_definition_bytes.value()[4..8] == &Schema::schema_schema_magic_bytes(),
                "pre existent save file database saved schema definition schema schema magic bytes wrong",
            );

            // decode saved schema
            let mut coder_state = CoderState::new(&schema_definition_schema, coder_state_alloc, None);
            let saved_schema_definition = decode_schema_definition(
                &mut Decoder::new(
                    &mut coder_state,
                    &mut &saved_schema_definition_bytes.value()[8..],
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
            let txn = db.begin_write()?;
            let mut table = txn.open_table(TABLE)?;
            table.insert(SCHEMA_DEFINITION_KEY, &buf.as_slice())?;
            drop(table);
            txn.commit()?;

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
        Ok(SaveDb {
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

    /// Read an entry from the save file by key.
    pub fn read<K: SaveKey>(&mut self, key: K) -> Result<Option<K::Val>> {
        // encode key into buf1
        self.buf1.clear();
        let mut coder_state = CoderState::new(
            &self.shared.key_schema,
            self.coder_state_alloc.take().unwrap_or_default(),
            None,
        );
        {
            let mut encoder = Encoder::new(&mut coder_state, &mut self.buf1);
            encoder.begin_enum(K::key_type_idx() + 1, K::key_type_name())?;
            key.encode(&mut encoder, &self.shared.game);
        }
        coder_state.is_finished_or_err()?;

        // get from database, short-circuit if None
        let txn = self.shared.db.begin_read()?;
        let table = txn.open_table(TABLE)?;
        let val_bytes = match table.get(&self.buf1.as_slice())? {
            Some(val) => val,
            None => {
                self.coder_state_alloc = Some(coder_state.into_alloc());
                return Ok(None)
            }
        };

        // decode val
        let mut coder_state = CoderState::new(
            &self.shared.val_schemas[K::key_type_idx()],
            coder_state.into_alloc(),
            None,
        );
        let val = <K as SaveKey>::Val::decode(
            &mut Decoder::new(&mut coder_state, &mut &*val_bytes.value()),
            &self.shared.game,
        )?;
        coder_state.is_finished_or_err()?;

        // done
        self.coder_state_alloc = Some(coder_state.into_alloc());
        Ok(Some(val))
    }

    /// Write/overwrite entries to the save file as an atomic transaction.
    pub fn write<I: IntoIterator<Item=SaveEntry>>(&mut self, entries: I) -> Result<()> {
        // write
        let mut coder_state_alloc = self.coder_state_alloc.take().unwrap_or_default();
        let txn = self.shared.db.begin_write()?;
        let mut table = txn.open_table(TABLE)?;

        for entry in entries {
            // encode key into buf1
            self.buf1.clear();
            let mut coder_state = CoderState::new(
                &self.shared.key_schema,
                coder_state_alloc,
                None,
            );
            {
                let mut encoder = Encoder::new(&mut coder_state, &mut self.buf1);
                encoder.begin_enum(entry.key_type_idx(), entry.key_type_name())?;
                entry.encode_key(&mut encoder, &self.shared.game)?;
            }
            coder_state.is_finished_or_err()?;

            // encode val into buf2
            self.buf2.clear();
            let mut coder_state = CoderState::new(
                &self.shared.val_schemas[entry.key_type_idx()],
                coder_state.into_alloc(),
                None,
            );
            entry.encode_val(
                &mut Encoder::new(&mut coder_state, &mut self.buf2),
                &self.shared.game,
            )?;
            coder_state.is_finished_or_err()?;

            // write key/value pair to write batch
            table.insert(&self.buf1.as_slice(), self.buf2.as_slice())?;

            // reset coder state alloc for next loop
            coder_state_alloc = coder_state.into_alloc();
        }

        // commit
        drop(table);
        txn.commit()?;

        // done
        self.coder_state_alloc = Some(coder_state_alloc);
        Ok(())
    }
}

impl Clone for SaveDb {
    fn clone(&self) -> Self {
        SaveDb {
            shared: Arc::clone(&self.shared),
            coder_state_alloc: None,
            buf1: Vec::new(),
            buf2: Vec::new(),
        }
    }
}


// ==== schema validation helper stuff ====

// internal representation of key/val schema definition
type SchemaDefinition = Vec<(String, Schema, Schema)>;

// pretty-print a key/val schema definition as a multi-line string
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

// schema for the key/val schema definition
fn schema_definition_schema() -> Schema {
    schema!(
        seq(varlen)(struct {
            (name: str),
            (key_schema: %Schema::schema_schema()),
            (val_schema: %Schema::schema_schema()),
        })
    )
}

// manually encode a key/val schema definition
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

// manually decode a key/val schema definition
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
