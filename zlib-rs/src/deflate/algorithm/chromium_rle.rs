#![forbid(unsafe_code)]

use super::flush_block;
use crate::{
    deflate::{fill_window, BlockState, DeflateStream, STD_MAX_MATCH, STD_MIN_MATCH},
    DeflateFlush,
};

/// Chromium zlib's `deflate_rle`: it refills the window whenever at most `STD_MAX_MATCH` bytes of
/// lookahead remain, and only emits a run once three bytes repeat the previous byte.
pub fn deflate_rle(stream: &mut DeflateStream, flush: DeflateFlush) -> BlockState {
    loop {
        if stream.state.lookahead <= STD_MAX_MATCH {
            fill_window(stream);
            if stream.state.lookahead <= STD_MAX_MATCH && flush == DeflateFlush::NoFlush {
                return BlockState::NeedMore;
            }
            if stream.state.lookahead == 0 {
                break;
            }
        }
        let state = &mut stream.state;
        let mut match_len = 0;
        if state.lookahead >= STD_MIN_MATCH && state.strstart > 0 {
            let window = state.window.filled();
            let prev = window[state.strstart - 1];
            let limit = Ord::min(STD_MAX_MATCH, state.lookahead);
            let run = window[state.strstart..state.strstart + limit]
                .iter()
                .take_while(|&&byte| byte == prev)
                .count();
            if run >= STD_MIN_MATCH {
                match_len = run;
            }
        }
        let bflush;
        if match_len >= STD_MIN_MATCH {
            bflush = state.tally_dist(1, match_len - STD_MIN_MATCH);
            state.lookahead -= match_len;
            state.strstart += match_len;
        } else {
            let lc = state.window.filled()[state.strstart];
            bflush = state.tally_lit(lc);
            state.lookahead -= 1;
            state.strstart += 1;
        }
        if bflush {
            flush_block!(stream, false);
        }
    }
    stream.state.insert = 0;
    if flush == DeflateFlush::Finish {
        flush_block!(stream, true);
        return BlockState::FinishDone;
    }
    if !stream.state.sym_buf.is_empty() {
        flush_block!(stream, false);
    }
    BlockState::BlockDone
}
