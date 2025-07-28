#[allow(clippy::manual_map)]
#[inline(always)]
pub fn select_first_some<V>(
    first_option: Option<V>,
    second_option: Option<V>,
) -> Option<V> {
    if let Some(first_some) = first_option {
        Some(first_some)
    } else if let Some(second_some) = second_option {
        Some(second_some)
    } else {
        None
    }
}


#[allow(clippy::manual_map)]
#[inline(always)]
pub fn select_first_some_3<V>(
    first_option: Option<V>,
    second_option: Option<V>,
    third_option: Option<V>,
) -> Option<V> {
    if let Some(first_some) = first_option {
        Some(first_some)
    } else if let Some(second_some) = second_option {
        Some(second_some)
    } else if let Some(third_some) = third_option {
        Some(third_some)
    } else {
        None
    }
}
