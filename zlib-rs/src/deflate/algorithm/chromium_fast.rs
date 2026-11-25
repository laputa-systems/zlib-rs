#![forbid(unsafe_code)]

use super::flush_block;
use crate::{
    deflate::{fill_window, BlockState, DeflateStream, MIN_LOOKAHEAD, STD_MIN_MATCH},
    DeflateFlush,
};

pub fn deflate_fast(stream: &mut DeflateStream, flush: DeflateFlush) -> BlockState {
    loop {
        if stream.state.lookahead < MIN_LOOKAHEAD {
            fill_window(stream);
            if stream.state.lookahead < MIN_LOOKAHEAD && flush == DeflateFlush::NoFlush {
                return BlockState::NeedMore;
            }
            if stream.state.lookahead == 0 { break; }
        }
        let state = &mut stream.state;
        let hash_head = if state.lookahead >= STD_MIN_MATCH {
            state.quick_insert_string(state.strstart)
        } else { 0 };
        let mut match_len = STD_MIN_MATCH - 1;
        if hash_head != 0 && state.strstart - hash_head as usize <= state.max_dist() {
            (match_len, state.match_start) = crate::deflate::longest_match::longest_match(state, hash_head);
        }
        let bflush;
        if match_len >= STD_MIN_MATCH {
            bflush = state.tally_dist(state.strstart - state.match_start as usize, match_len - STD_MIN_MATCH);
            state.lookahead -= match_len;
            if match_len <= state.max_insert_length() && state.lookahead >= STD_MIN_MATCH {
                match_len -= 1;
                while match_len != 0 {
                    state.strstart += 1;
                    state.quick_insert_string(state.strstart);
                    match_len -= 1;
                }
                state.strstart += 1;
            } else {
                state.strstart += match_len;
            }
        } else {
            bflush = state.tally_lit(state.window.filled()[state.strstart]);
            state.lookahead -= 1;
            state.strstart += 1;
        }
        if bflush { flush_block!(stream, false); }
    }
    stream.state.insert = Ord::min(stream.state.strstart, STD_MIN_MATCH - 1);
    if flush == DeflateFlush::Finish {
        flush_block!(stream, true);
        return BlockState::FinishDone;
    }
    if !stream.state.sym_buf.is_empty() { flush_block!(stream, false); }
    BlockState::BlockDone
}
