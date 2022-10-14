use neon::prelude::*;

use crate::db::traits::{JsNewWithArcMutex, JsNewWithBox, JsNewWithBoxRef};
use crate::db::types::DbOptions;

pub mod batch;
pub mod consts;
pub mod db;
pub mod smt;
pub mod smt_db;
pub mod types;

mod codec;
mod database;
mod diff;
mod in_memory_db;
mod in_memory_smt;
mod read_writer_db;
mod reader_db;
mod state_db;
mod state_writer;
mod utils;

use batch::WriteBatch;
use database::Database;
use in_memory_smt::InMemorySMT;
use read_writer_db::ReadWriter;
use state_db::StateDB;
use state_writer::StateWriter;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("db_new", Database::js_new_with_box::<DbOptions, Database>)?;
    cx.export_function("db_clear", Database::js_clear)?;
    cx.export_function("db_close", Database::js_close)?;
    cx.export_function("db_get", Database::js_get)?;
    cx.export_function("db_exists", Database::js_exists)?;
    cx.export_function("db_set", Database::js_set)?;
    cx.export_function("db_del", Database::js_del)?;
    cx.export_function("db_write", Database::js_write)?;
    cx.export_function("db_iterate", Database::js_iterate)?;
    cx.export_function("db_checkpoint", Database::js_checkpoint)?;

    cx.export_function("state_db_reader_new", reader_db::Reader::js_new)?;
    cx.export_function("state_db_reader_close", reader_db::Reader::js_close)?;
    cx.export_function("state_db_reader_get", reader_db::Reader::js_get)?;
    cx.export_function("state_db_reader_exists", reader_db::Reader::js_exists)?;
    cx.export_function("state_db_reader_iterate", reader_db::Reader::js_iterate)?;

    cx.export_function("state_db_read_writer_new", ReadWriter::js_new)?;
    cx.export_function("state_db_read_writer_close", ReadWriter::js_close)?;
    cx.export_function("state_db_read_writer_upsert_key", ReadWriter::js_upsert_key)?;
    cx.export_function("state_db_read_writer_get_key", ReadWriter::js_get_key)?;
    cx.export_function("state_db_read_writer_delete", ReadWriter::js_delete_key)?;
    cx.export_function("state_db_read_writer_range", ReadWriter::js_range)?;

    cx.export_function("batch_new", WriteBatch::js_new_with_arc_mutex::<WriteBatch>)?;
    cx.export_function("batch_set", WriteBatch::js_set)?;
    cx.export_function("batch_del", WriteBatch::js_del)?;

    let state_db_new = StateDB::js_new_with_box_ref::<DbOptions, StateDB>;
    cx.export_function("state_db_new", state_db_new)?;
    cx.export_function("state_db_get_current_state", StateDB::js_get_current_state)?;
    cx.export_function("state_db_close", StateDB::js_close)?;
    cx.export_function("state_db_get", StateDB::js_get)?;
    cx.export_function("state_db_exists", StateDB::js_exists)?;
    cx.export_function("state_db_iterate", StateDB::js_iterate)?;
    cx.export_function("state_db_revert", StateDB::js_revert)?;
    cx.export_function("state_db_commit", StateDB::js_commit)?;
    cx.export_function("state_db_prove", StateDB::js_prove)?;
    cx.export_function("state_db_verify", StateDB::js_verify)?;
    cx.export_function("state_db_clean_diff_until", StateDB::js_clean_diff_until)?;
    cx.export_function("state_db_checkpoint", StateDB::js_checkpoint)?;

    let state_writer_new = StateWriter::js_new_with_arc_mutex::<StateWriter>;
    let restore_snapshot = StateWriter::js_restore_snapshot;
    cx.export_function("state_writer_new", state_writer_new)?;
    cx.export_function("state_writer_snapshot", StateWriter::js_snapshot)?;
    cx.export_function("state_writer_restore_snapshot", restore_snapshot)?;

    cx.export_function("in_memory_db_new", in_memory_db::Database::js_new)?;
    cx.export_function("in_memory_db_clone", in_memory_db::Database::js_clone)?;
    cx.export_function("in_memory_db_get", in_memory_db::Database::js_get)?;
    cx.export_function("in_memory_db_set", in_memory_db::Database::js_set)?;
    cx.export_function("in_memory_db_del", in_memory_db::Database::js_del)?;
    cx.export_function("in_memory_db_clear", in_memory_db::Database::js_clear)?;
    cx.export_function("in_memory_db_write", in_memory_db::Database::js_write)?;
    cx.export_function("in_memory_db_iterate", in_memory_db::Database::js_iterate)?;

    let in_memory_smt_new = InMemorySMT::js_new_with_arc_mutex::<InMemorySMT>;
    cx.export_function("in_memory_smt_new", in_memory_smt_new)?;
    cx.export_function("in_memory_smt_update", InMemorySMT::js_update)?;
    cx.export_function("in_memory_smt_prove", InMemorySMT::js_prove)?;
    cx.export_function("in_memory_smt_verify", InMemorySMT::js_verify)?;

    Ok(())
}
