//! LayoutCache — manages cached line layouts for the editor.
//!
//! Extracted from lines.rs to follow Single Responsibility Principle.
//! Handles caching of Parley line layouts with revision tracking.

use std::fmt;

use crate::text_layout::LineLayout;

/// Cache for computed line layouts.
///
/// Each buffer line can have a cached layout. Layouts are computed lazily
/// (only for visible lines) and invalidated when content changes.
pub struct LayoutCache {
    /// Per-line layout cache. Indexed by buffer line number.
    /// `None` means the layout hasn't been computed yet (or was invalidated).
    layouts: Vec<Option<LineLayout>>,

    /// Estimated line height in pixels (used when actual layout isn't computed).
    /// Set from the first computed layout, or from font_size * 1.2 as default.
    line_height: f64,
}

impl fmt::Debug for LayoutCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cached_count = self.layouts.iter().filter(|l| l.is_some()).count();
        f.debug_struct("LayoutCache")
            .field("line_count", &self.layouts.len())
            .field("cached_layouts", &cached_count)
            .field("line_height", &self.line_height)
            .finish()
    }
}

impl LayoutCache {
    /// Create a new layout cache for the given number of lines.
    pub fn new(line_count: usize, default_line_height: f64) -> Self {
        let mut layouts = Vec::with_capacity(line_count);
        layouts.resize_with(line_count, || None);
        Self {
            layouts,
            line_height: default_line_height,
        }
    }

    /// Get the estimated line height.
    pub fn line_height(&self) -> f64 {
        self.line_height
    }

    /// Update the estimated line height.
    pub fn set_line_height(&mut self, height: f64) {
        self.line_height = height;
    }

    /// Get the number of lines in the cache.
    pub fn line_count(&self) -> usize {
        self.layouts.len()
    }

    /// Resize the cache to match a new line count.
    ///
    /// New lines are initialized with no layout.
    pub fn resize(&mut self, new_line_count: usize) {
        self.layouts.resize_with(new_line_count, || None);
    }

    /// Get a reference to the cached layout for a line (if it exists).
    pub fn get(&self, line: usize) -> Option<&LineLayout> {
        self.layouts.get(line)?.as_ref()
    }

    /// Get a mutable reference to the cached layout for a line (if it exists).
    pub fn get_mut(&mut self, line: usize) -> Option<&mut LineLayout> {
        self.layouts.get_mut(line)?.as_mut()
    }

    /// Store a layout for a line.
    ///
    /// Also updates the estimated line height if this is a single-line layout.
    pub fn set(&mut self, line: usize, layout: LineLayout) {
        if line >= self.layouts.len() {
            return;
        }

        // Update estimated line height from actual computed layout
        if layout.visual_line_count == 1 {
            self.line_height = layout.line_height;
        }

        self.layouts[line] = Some(layout);
    }

    /// Check if a layout exists and is valid for the given revision.
    pub fn is_valid(&self, line: usize, doc_revision: u64) -> bool {
        self.layouts
            .get(line)
            .and_then(|opt| opt.as_ref())
            .map(|layout| layout.revision == doc_revision)
            .unwrap_or(false)
    }

    /// Invalidate the cached layout for a single line.
    pub fn invalidate(&mut self, line: usize) {
        if line < self.layouts.len() {
            self.layouts[line] = None;
        }
    }

    /// Invalidate all cached layouts.
    pub fn invalidate_all(&mut self) {
        for slot in &mut self.layouts {
            *slot = None;
        }
    }

    /// Get the height of a specific line.
    ///
    /// Returns the actual height if layout is cached, otherwise the estimated height.
    pub fn height_of(&self, line: usize) -> f64 {
        self.layouts
            .get(line)
            .and_then(|opt| opt.as_ref())
            .map(|layout| layout.line_height)
            .unwrap_or(self.line_height)
    }

    /// Compute the total height of all lines.
    ///
    /// Uses actual heights for cached layouts, estimated height for others.
    pub fn total_height(&self) -> f64 {
        self.layouts
            .iter()
            .map(|slot| {
                slot.as_ref()
                    .map(|layout| layout.line_height)
                    .unwrap_or(self.line_height)
            })
            .sum()
    }

    /// Compute the maximum width across all cached layouts.
    ///
    /// Only considers lines that have been laid out; uncached lines contribute 0.
    pub fn max_width(&self) -> f64 {
        self.layouts
            .iter()
            .filter_map(|slot| slot.as_ref())
            .map(|layout| layout.layout.width() as f64)
            .fold(0.0_f64, f64::max)
    }

    /// Count the number of visual lines (for wrapped layouts).
    pub fn total_visual_lines(&self) -> usize {
        self.layouts
            .iter()
            .map(|slot| {
                slot.as_ref()
                    .map(|layout| layout.visual_line_count)
                    .unwrap_or(1)
            })
            .sum()
    }

    /// Iterate over all cached layouts with their line indices.
    pub fn iter_cached(&self) -> impl Iterator<Item = (usize, &LineLayout)> {
        self.layouts
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|layout| (i, layout)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cache() {
        let cache = LayoutCache::new(100, 16.0);
        assert_eq!(cache.line_count(), 100);
        assert_eq!(cache.line_height(), 16.0);
        assert!(cache.get(0).is_none());
    }

    #[test]
    fn test_resize() {
        let mut cache = LayoutCache::new(10, 16.0);
        assert_eq!(cache.line_count(), 10);

        cache.resize(20);
        assert_eq!(cache.line_count(), 20);

        cache.resize(5);
        assert_eq!(cache.line_count(), 5);
    }

    #[test]
    fn test_invalidate() {
        let mut cache = LayoutCache::new(10, 16.0);

        // Can invalidate without error even if no layout exists
        cache.invalidate(5);
        assert!(cache.get(5).is_none());

        // Invalidate all
        cache.invalidate_all();
        for i in 0..10 {
            assert!(cache.get(i).is_none());
        }
    }

    #[test]
    fn test_height_calculations() {
        let cache = LayoutCache::new(5, 20.0);

        // Without any cached layouts, uses estimated height
        assert_eq!(cache.height_of(0), 20.0);
        assert_eq!(cache.total_height(), 100.0); // 5 * 20
    }
}
