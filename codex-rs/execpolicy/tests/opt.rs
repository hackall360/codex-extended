use codex_execpolicy::{Opt, OptMeta};
use starlark::values::{Heap, UnpackValue, ValueLike};

#[test]
fn unpack_borrowed_opt() {
    let heap = Heap::new();
    let value = heap.alloc(Opt::new("-h".to_owned(), OptMeta::Flag, true));

    let direct = value.downcast_ref::<Opt>().unwrap();
    let unpacked = <&Opt as UnpackValue>::unpack_value(value).unwrap().unwrap();

    assert!(std::ptr::eq(direct, unpacked));
    let owned = unpacked.to_owned();
    assert_eq!(owned, *unpacked);
}
