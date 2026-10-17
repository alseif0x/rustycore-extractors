use super::*;

fn order(values: &[u16], unique: usize) -> Vec<u16> {
    let mut state = Order::new(unique, 2 * 1024 * 1024).unwrap();
    for value in values {
        state.insert(*value).unwrap();
    }
    state.finish().unwrap()
}

#[test]
fn empty_needs_no_heap_budget() {
    assert!(Order::new(0, 0).unwrap().finish().unwrap().is_empty());
}
#[test]
fn n03_native_two_children() {
    assert_eq!(order(&[0, 3], 2), [3, 0]);
}
#[test]
fn duplicates_do_not_move_nodes() {
    assert_eq!(order(&[0, 3, 0, 3], 2), [3, 0]);
}
#[test]
fn duplicate_at_threshold_does_not_rehash() {
    let mut state = Order::new(14, 2 * 1024 * 1024).unwrap();
    for value in 0..13 {
        assert!(state.insert(value).unwrap());
    }
    assert_eq!(state.bucket_count, 13);
    assert!(!state.insert(0).unwrap());
    assert_eq!(state.bucket_count, 13);
    assert!(state.insert(13).unwrap());
    assert_eq!(state.bucket_count, 29);
    let mut expected = vec![13];
    expected.extend(0..13);
    assert_eq!(state.finish().unwrap(), expected);
}
#[test]
fn bucket_collision_is_not_global_reverse() {
    assert_eq!(order(&[0, 1, 13], 3), [1, 13, 0]);
}
#[test]
fn first_rehash_relinks_existing_nodes() {
    let input: Vec<u16> = (0..14).collect();
    let mut expected = vec![13];
    expected.extend(0..13);
    assert_eq!(order(&input, 14), expected);
}
#[test]
fn collision_chain_survives_rehash() {
    let input: Vec<u16> = (0..14).map(|n| n * 13).collect();
    let mut expected = vec![169];
    expected.extend((0..13).map(|n| n * 13));
    assert_eq!(order(&input, 14), expected);
}
#[test]
fn rejects_excess_domain_before_allocation() {
    assert!(matches!(Order::new(65537, usize::MAX), Err(Error::Limit)));
}
#[test]
fn exact_declared_heap_ceiling() {
    let minimum = 13 * size_of::<usize>() + size_of::<Node>() + size_of::<u16>();
    assert!(matches!(Order::new(1, minimum - 1), Err(Error::Limit)));
    let mut state = Order::new(1, minimum).unwrap();
    state.insert(0).unwrap();
    assert_eq!(state.finish().unwrap(), [0]);
}
#[test]
fn work_charge_includes_returned_vector_capacity() {
    let mut state = Order::new(2, 2 * 1024 * 1024).unwrap();
    let charge = state.work_bytes().unwrap();
    let output_charge = state.output.capacity() * size_of::<u16>();
    assert_eq!(
        charge,
        state.buckets.capacity() * size_of::<usize>()
            + state.nodes.capacity() * size_of::<Node>()
            + output_charge
    );
    state.insert(0).unwrap();
    state.insert(3).unwrap();
    assert_eq!(state.work_bytes().unwrap(), charge);
    let output = state.finish().unwrap();
    assert_eq!(output.capacity() * size_of::<u16>(), output_charge);
}
#[test]
fn extra_unique_reference_rejects() {
    let mut state = Order::new(1, 1024).unwrap();
    state.insert(0).unwrap();
    assert!(!state.insert(0).unwrap());
    assert!(matches!(state.insert(1), Err(Error::State)));
}
#[test]
fn incomplete_source_selection_rejects() {
    let mut state = Order::new(2, 1024).unwrap();
    state.insert(0).unwrap();
    assert!(matches!(state.finish(), Err(Error::State)));
}
#[test]
fn full_u16_domain_is_a_permutation() {
    let input: Vec<u16> = (0..=u16::MAX).collect();
    let mut result = order(&input, 65536);
    result.sort_unstable();
    assert_eq!(result, input);
}
#[test]
fn zero_reference_owner_rejects_insert() {
    assert!(matches!(
        Order::new(0, 0).unwrap().insert(0),
        Err(Error::State)
    ));
}
