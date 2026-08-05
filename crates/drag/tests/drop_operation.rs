// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use drag::DropOperation;

#[test]
fn mask_operations() {
    let mask = DropOperation::COPY | DropOperation::LINK;
    assert!(mask.intersects(DropOperation::COPY));
    assert!(mask.intersects(DropOperation::LINK));
    assert!(!mask.intersects(DropOperation::MOVE));
    assert!(!mask.is_empty());
    assert_eq!(mask.bits(), 0b101);

    assert!(DropOperation::NONE.is_empty());
    assert!(!DropOperation::NONE.intersects(mask));

    let mut mask = DropOperation::NONE;
    mask |= DropOperation::MOVE;
    assert_eq!(mask, DropOperation::MOVE);
}

#[test]
fn from_bits_truncate_drops_undefined_bits() {
    assert_eq!(
        DropOperation::from_bits_truncate(u32::MAX),
        DropOperation::COPY | DropOperation::MOVE | DropOperation::LINK
    );
    assert_eq!(DropOperation::from_bits_truncate(0), DropOperation::NONE);
    assert_eq!(
        DropOperation::from_bits_truncate(DropOperation::MOVE.bits()),
        DropOperation::MOVE
    );
}

#[cfg(feature = "serde")]
mod serde_wire {
    use drag::{DragResult, DropOperation};

    #[test]
    fn drop_operation_is_a_bare_number_on_the_wire() {
        let mask = DropOperation::COPY | DropOperation::LINK;
        assert_eq!(serde_json::to_string(&mask).unwrap(), "5");
        let back: DropOperation = serde_json::from_str("5").unwrap();
        assert_eq!(back, mask);
    }

    #[test]
    fn deserialization_truncates_undefined_bits() {
        // Without routing through `from_bits_truncate`, this would build a
        // mask that intersects everything.
        let back: DropOperation = serde_json::from_str("4294967295").unwrap();
        assert_eq!(
            back,
            DropOperation::COPY | DropOperation::MOVE | DropOperation::LINK
        );
    }

    #[test]
    fn drag_result_wire_shape() {
        let dropped = DragResult::Dropped(DropOperation::MOVE);
        assert_eq!(serde_json::to_string(&dropped).unwrap(), r#"{"Dropped":2}"#);
        let cancel = DragResult::Cancel;
        assert_eq!(serde_json::to_string(&cancel).unwrap(), r#""Cancel""#);
    }
}
