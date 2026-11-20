//! Fission async `JobSpec`s. One per network call.
//!
//! Each is registered at app startup via
//! `WebApp::with_async(|r| r.register_job(JOB, handler))`. Reducers launch them
//! with `ctx.effects.app(JOB, request).on_ok(...).on_err(...).dispatch()`.

use fission::core::{JobRef, JobSpec};
use shared::{BookSnapshot, BookUpdate, NewOrder, Symbol};

pub struct FetchSymbols;
impl JobSpec for FetchSymbols {
    type Request = ();
    type Ok = Vec<Symbol>;
    type Err = String;
    const NAME: &'static str = "fetch_symbols";
}
pub const FETCH_SYMBOLS: JobRef<FetchSymbols> = JobRef::new("fetch_symbols");

pub struct FetchSnapshot;
impl JobSpec for FetchSnapshot {
    type Request = Symbol;
    type Ok = BookSnapshot;
    type Err = String;
    const NAME: &'static str = "fetch_snapshot";
}
pub const FETCH_SNAPSHOT: JobRef<FetchSnapshot> = JobRef::new("fetch_snapshot");

pub struct PostOrder;
impl JobSpec for PostOrder {
    type Request = NewOrder;
    type Ok = BookUpdate;
    type Err = String;
    const NAME: &'static str = "post_order";
}
pub const POST_ORDER: JobRef<PostOrder> = JobRef::new("post_order");
