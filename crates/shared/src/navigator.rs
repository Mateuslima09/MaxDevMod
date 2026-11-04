//! Where the game's navigator keeps its routing mode and its four lists of targets, read from its "set targets"
//! function (the plugin calls it: crates/plugin navigation.rs; the host reads the waypoints: crates/host
//! route.rs), not fixed: ATS 1.62 moved them all 0x10 on (owner 2026-11-03, ETS2 and ATS 1.61 at `+0x1C8`).
//!
//! The function saves its registers, keeps the mode (`mov [rcx+mode], edx`) and starts on the first list
//! (`add rcx, planned`), then the three others (`lea rcx, [rdi+list]`): the route, the places to avoid and the
//! waypoints, each list 0x28 bytes after the one before.

/// The navigator's fields, as offsets in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigatorLists {
    /// The routing mode (a `u32`).
    pub mode: usize,
    /// The route as the game plans it (given empty).
    pub planned: usize,
    /// The waypoints in route order, then a job's destination.
    pub route: usize,
    /// The places to avoid.
    pub avoid: usize,
    /// The waypoints, in the order they were added.
    pub waypoints: usize,
}

/// The function's start, the mode's and the first list's offsets left open (`None`).
const START: [Option<u8>; 31] = [
    Some(0x48),
    Some(0x89),
    Some(0x5c),
    Some(0x24),
    Some(0x08), // mov [rsp+8], rbx
    Some(0x48),
    Some(0x89),
    Some(0x74),
    Some(0x24),
    Some(0x10), // mov [rsp+10h], rsi
    Some(0x57), // push rdi
    Some(0x48),
    Some(0x83),
    Some(0xec),
    Some(0x20), // sub rsp, 20h
    Some(0x48),
    Some(0x8b),
    Some(0xf9), // mov rdi, rcx
    Some(0x89),
    Some(0x91),
    None,
    None,
    Some(0x00),
    Some(0x00), // mov [rcx+mode], edx
    Some(0x48),
    Some(0x81),
    Some(0xc1),
    None,
    None,
    Some(0x00),
    Some(0x00), // add rcx, planned
];
/// How far after its start the function's other lists are named.
const LISTS_WITHIN: usize = 0x90;
/// Each list is an `array_t` and a little more: the next one this far on.
const LIST_STEP: usize = 0x28;

/// The "set targets" function in `code`, if exactly once: where it starts in `code`, and the navigator's fields
/// it names, if they're laid out as described.
pub fn find_set_targets(code: &[u8]) -> Option<(usize, NavigatorLists)> {
    let fits =
        |at: usize| code[at..at + START.len()].iter().zip(START).all(|(byte, want)| want.is_none_or(|w| *byte == w));
    let mut found = (0..code.len().saturating_sub(START.len())).filter(|&at| code[at] == 0x48 && fits(at));
    let at = found.next()?;
    if found.next().is_some() {
        return None;
    }
    let u32_at = |i: usize| code.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let (mode, planned) = (u32_at(at + 20)?, u32_at(at + 27)?);
    // `lea rcx, [rdi+imm32]`, the next three after the start.
    let body = code.get(at + START.len()..at + LISTS_WITHIN)?;
    let mut leas = (0..body.len().saturating_sub(7)).filter(|&i| body[i..i + 3] == [0x48, 0x8d, 0x8f]);
    let mut next = || leas.next().and_then(|i| u32_at(at + START.len() + i + 3));
    let (route, avoid, waypoints) = (next()?, next()?, next()?);
    let lists = NavigatorLists { mode, planned, route, avoid, waypoints };
    let laid_out =
        mode < planned && route == planned + LIST_STEP && avoid == route + LIST_STEP && waypoints == avoid + LIST_STEP;
    laid_out.then_some((at, lists))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace().map(|byte| u8::from_str_radix(byte, 16).unwrap()).collect()
    }

    /// ETS2 1.61.1.1's function's first 160 bytes.
    const ETS2_1_61: &str = "48 89 5c 24 08 48 89 74 24 10 57 48 83 ec 20 48 8b f9 89 91 c8 01 00 00 48 81 c1 d0 01 00 00 33 f6 49 8b d9 4c 3b c1 74 0c 49 8b d0 48 89 71 10 e8 db 3a 00 00 48 8d 8f f8 01 00 00 48 3b d9 74 0c 48 8b d3 48 89 71 10 e8 c3 3a 00 00 48 8b 54 24 50 48 8d 8f 20 02 00 00 48 3b d1 74 09 48 89 71 10 e8 a9 3a 00 00 48 8b 54 24 58 48 8d 8f 48 02 00 00 48 3b d1 74 09 48 89 71 10 e8 8f 3a 00 00 ff 87 70 02 00 00 48 8d 0d 02 d1 b4 02 40 88 b7 a8 01 00 00 ff 15 75 3a 8c 01 48 8b 05 06 49";
    /// ATS 1.62.1.1's: the same, every field 0x10 on.
    const ATS_1_62: &str = "48 89 5c 24 08 48 89 74 24 10 57 48 83 ec 20 48 8b f9 89 91 d8 01 00 00 48 81 c1 e0 01 00 00 33 f6 49 8b d9 4c 3b c1 74 0c 49 8b d0 48 89 71 10 e8 2b 3c 00 00 48 8d 8f 08 02 00 00 48 3b d9 74 0c 48 8b d3 48 89 71 10 e8 13 3c 00 00 48 8b 54 24 50 48 8d 8f 30 02 00 00 48 3b d1 74 09 48 89 71 10 e8 f9 3b 00 00 48 8b 54 24 58 48 8d 8f 58 02 00 00 48 3b d1 74 09 48 89 71 10 e8 df 3b 00 00 ff 87 80 02 00 00 48 8d 0d b2 b2 ba 02 40 88 b7 b8 01 00 00 ff 15 45 c5 8e 01 48 8b 05 ae 34";

    #[test]
    fn reads_the_navigators_fields_from_either_version() {
        let code = [vec![0xcc; 16], hex(ETS2_1_61)].concat();
        let lists = NavigatorLists { mode: 0x1c8, planned: 0x1d0, route: 0x1f8, avoid: 0x220, waypoints: 0x248 };
        assert_eq!(find_set_targets(&code), Some((16, lists)));
        let lists = NavigatorLists { mode: 0x1d8, planned: 0x1e0, route: 0x208, avoid: 0x230, waypoints: 0x258 };
        assert_eq!(find_set_targets(&hex(ATS_1_62)), Some((0, lists)));
    }

    #[test]
    fn finds_nothing_twice_over_or_laid_out_otherwise() {
        let twice = [hex(ETS2_1_61), hex(ATS_1_62)].concat();
        assert_eq!(find_set_targets(&twice), None);
        // The waypoints' list not where the others put it.
        let moved = hex(&ETS2_1_61.replace("8f 48 02", "8f 50 02"));
        assert_eq!(find_set_targets(&moved), None);
        assert_eq!(find_set_targets(&[]), None);
    }
}
