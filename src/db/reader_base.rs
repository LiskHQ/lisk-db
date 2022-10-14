use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;

use neon::context::{Context, FunctionContext};
use neon::event::Channel;
use neon::handle::{Handle, Root};
use neon::result::JsResult;
use neon::types::{Finalize, JsBuffer, JsFunction, JsValue};

use crate::db::types::{JsBoxRef, Kind, SnapshotMessage};
use crate::state_db::SharedStateDB;

pub struct ReaderBase {
    tx: mpsc::Sender<SnapshotMessage>,
}

impl Finalize for ReaderBase {}
impl ReaderBase {
    pub fn js_new(mut ctx: FunctionContext) -> JsResult<JsBoxRef<Self>> {
        // Channel for sending callbacks to execute on the sqlite connection thread
        let (tx, rx) = mpsc::channel::<SnapshotMessage>();
        let channel = ctx.channel();

        let db = ctx
            .argument::<SharedStateDB>(0)?
            .downcast_or_throw::<SharedStateDB, _>(&mut ctx)?;
        let db = db.borrow();
        let conn = db.arc_clone();
        thread::spawn(move || {
            let snapshot = conn.snapshot();
            while let Ok(message) = rx.recv() {
                match message {
                    SnapshotMessage::Callback(f) => {
                        f(&snapshot, &channel);
                    },
                    SnapshotMessage::Close => return,
                }
            }
        });

        Ok(ctx.boxed(RefCell::new(Self { tx })))
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a closed database
    pub fn close(&self) -> Result<(), mpsc::SendError<SnapshotMessage>> {
        self.tx.send(SnapshotMessage::Close)
    }

    pub fn send(
        &self,
        callback: impl FnOnce(&rocksdb::Snapshot, &Channel) + Send + 'static,
    ) -> Result<(), mpsc::SendError<SnapshotMessage>> {
        self.tx.send(SnapshotMessage::Callback(Box::new(callback)))
    }

    pub fn get_by_key(
        &self,
        key: Vec<u8>,
        cb: Root<JsFunction>,
    ) -> Result<(), mpsc::SendError<SnapshotMessage>> {
        let key = Kind::State.key(key);
        self.send(move |conn, channel| {
            let result = conn.get(&key);

            channel.send(move |mut ctx| {
                let callback = cb.into_inner(&mut ctx);
                let this = ctx.undefined();
                let args: Vec<Handle<JsValue>> = match result {
                    Ok(Some(val)) => {
                        let buffer = JsBuffer::external(&mut ctx, val);
                        vec![ctx.null().upcast(), buffer.upcast()]
                    },
                    Ok(None) => vec![ctx.error("No data")?.upcast()],
                    Err(err) => vec![ctx.error(&err)?.upcast()],
                };

                callback.call(&mut ctx, this, args)?;

                Ok(())
            });
        })
    }
}
