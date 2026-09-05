use super::*;

// ==================== Display Order Tests ====================

#[test]
fn test_flip_index_reverses_row_order() {
    // Queue [old, mid, new] displays as [new, mid, old]
    assert_eq!(flip_index(0, 3), Some(2));
    assert_eq!(flip_index(1, 3), Some(1));
    assert_eq!(flip_index(2, 3), Some(0));
}

#[test]
fn test_flip_index_is_its_own_inverse() {
    for index in 0..5 {
        let flipped = flip_index(index, 5).expect("row is within the queue");
        assert_eq!(flip_index(flipped, 5), Some(index));
    }
}

#[test]
fn test_flip_index_out_of_range() {
    assert_eq!(flip_index(0, 0), None);
    assert_eq!(flip_index(3, 3), None);
}
