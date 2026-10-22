// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Bounded body assembly that retains a single owned HTTP chunk.

/// Keep the transport's chunk type until concatenation is actually required.
/// The private generic also permits ownership and limit tests with ordinary
/// byte buffers, without exposing a transport buffer type in the public API.
pub(crate) enum ResponseBody<C> {
    Empty,
    Single(C),
    Buffered(Vec<u8>),
}

impl<C: AsRef<[u8]>> ResponseBody<C> {
    pub fn new() -> Self {
        Self::Empty
    }

    /// Reject an oversized chunk before modifying the accumulated bytes.
    /// Reserve from the bounded length hint only when a second nonempty chunk
    /// makes contiguous owned storage necessary.
    pub fn append(&mut self, chunk: C, limit: usize, capacity_hint: usize) -> bool {
        let Some(combined) = self.as_slice().len().checked_add(chunk.as_ref().len()) else {
            return false;
        };
        if combined > limit {
            return false;
        }
        if chunk.as_ref().is_empty() {
            return true;
        }
        match self {
            Self::Empty => *self = Self::Single(chunk),
            Self::Single(first) => {
                let capacity = combined.max(capacity_hint.min(64 * 1024).min(limit));
                let mut bytes = Vec::with_capacity(capacity);
                bytes.extend_from_slice(first.as_ref());
                bytes.extend_from_slice(chunk.as_ref());
                *self = Self::Buffered(bytes);
            }
            Self::Buffered(bytes) => bytes.extend_from_slice(chunk.as_ref()),
        }
        true
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Empty => &[],
            Self::Single(chunk) => chunk.as_ref(),
            Self::Buffered(bytes) => bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn single_chunk_preserves_the_original_allocation() {
        let chunk = vec![0, 1, 255, 0];
        let original = chunk.as_ptr();
        let mut body = ResponseBody::new();
        assert!(body.append(chunk, 4, usize::MAX));
        assert_eq!(body.as_slice(), [0, 1, 255, 0]);
        assert_eq!(body.as_slice().as_ptr(), original);
        assert!(matches!(body, ResponseBody::Single(_)));
    }

    #[test]
    fn empty_chunks_do_not_force_buffering() {
        let mut body = ResponseBody::new();
        assert!(body.append(Vec::new(), 4, usize::MAX));
        assert!(matches!(body, ResponseBody::Empty));
        let chunk = vec![1, 2];
        let original = chunk.as_ptr();
        assert!(body.append(chunk, 4, usize::MAX));
        assert!(body.append(Vec::new(), 4, usize::MAX));
        assert_eq!(body.as_slice().as_ptr(), original);
    }

    #[test]
    fn multiple_chunks_concatenate_and_respect_capacity_and_length_bounds() {
        let mut body = ResponseBody::new();
        assert!(body.append(vec![0, 255], 6, usize::MAX));
        assert!(body.append(vec![1, 2], 6, usize::MAX));
        assert!(body.append(vec![3, 4], 6, usize::MAX));
        assert_eq!(body.as_slice(), [0, 255, 1, 2, 3, 4]);
        assert!(matches!(body, ResponseBody::Buffered(_)));
        assert!(!body.append(vec![5], 6, usize::MAX));
        assert_eq!(body.as_slice(), [0, 255, 1, 2, 3, 4]);
    }

    #[test]
    fn an_oversized_first_chunk_does_not_replace_the_empty_body() {
        let mut body = ResponseBody::new();
        assert!(!body.append(vec![1], 0, usize::MAX));
        assert!(matches!(body, ResponseBody::Empty));
        assert!(body.append(Vec::new(), 0, usize::MAX));
    }

    #[test]
    fn coalescing_releases_the_original_transport_chunk() {
        let first: Arc<[u8]> = Arc::from(&b"first"[..]);
        let original = Arc::downgrade(&first);
        let mut body = ResponseBody::new();
        assert!(body.append(first, 32, 32));
        assert!(original.upgrade().is_some());
        assert!(body.append(Arc::from(&b"second"[..]), 32, 32));
        assert!(original.upgrade().is_none());
        assert_eq!(body.as_slice(), b"firstsecond");
    }
}
