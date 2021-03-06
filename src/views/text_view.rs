use Printer;
use With;
use XY;
use align::*;
use direction::Direction;
use event::*;

use unicode_width::UnicodeWidthStr;

use utils::{LinesIterator, Row};
use vec::Vec2;
use view::{SizeCache, View};
use view::ScrollBase;


/// A simple view showing a fixed text
pub struct TextView {
    content: String,
    rows: Vec<Row>,

    align: Align,

    // If `false`, disable scrolling.
    scrollable: bool,

    // ScrollBase make many scrolling-related things easier
    scrollbase: ScrollBase,
    last_size: Option<XY<SizeCache>>,
    width: Option<usize>,
}

// If the last character is a newline, strip it.
fn strip_last_newline(content: &mut String) {
    if content.ends_with('\n') {
        content.pop().unwrap();
    }
}

impl TextView {
    /// Creates a new TextView with the given content.
    pub fn new<S: Into<String>>(content: S) -> Self {
        let mut content = content.into();
        strip_last_newline(&mut content);
        TextView {
            content: content,
            rows: Vec::new(),
            scrollable: true,
            scrollbase: ScrollBase::new(),
            align: Align::top_left(),
            last_size: None,
            width: None,
        }
    }

    /// Enable or disable the view's scrolling capabilities.
    ///
    /// When disabled, the view will never attempt to scroll
    /// (and will always ask for the full height).
    pub fn set_scrollable(&mut self, scrollable: bool) {
        self.scrollable = scrollable;
    }

    /// Enable or disable the view's scrolling capabilities.
    ///
    /// When disabled, the view will never attempt to scroll
    /// (and will always ask for the full height).
    ///
    /// Chainable variant.
    pub fn scrollable(self, scrollable: bool) -> Self {
        self.with(|s| s.set_scrollable(scrollable))
    }

    /// Sets the horizontal alignment for this view.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Sets the vertical alignment for this view.
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Sets the alignment for this view.
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;

        self
    }

    /// Center the text horizontally and vertically inside the view.
    pub fn center(mut self) -> Self {
        self.align = Align::center();
        self
    }

    /// Replace the text in this view.
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        let mut content = content.into();
        strip_last_newline(&mut content);
        self.content = content;
        self.invalidate();
    }

    /// Returns the current text in this view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        match self.last_size {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
    }

    fn compute_rows(&mut self, size: Vec2) {
        if !self.is_cache_valid(size) {
            self.last_size = None;
            // Recompute

            if size.x == 0 {
                // Nothing we can do at this poing.
                return;
            }

            self.rows = LinesIterator::new(&self.content, size.x).collect();
            let mut scrollbar = 0;
            if self.scrollable && self.rows.len() > size.y {
                scrollbar = 2;
                if size.x < scrollbar {
                    // Again, this is a lost cause.
                    return;
                }

                // If we're too high, include a scrollbar
                self.rows = LinesIterator::new(&self.content,
                                               size.x - scrollbar)
                    .collect();
                if self.rows.is_empty() && !self.content.is_empty() {
                    return;
                }
            }

            // Desired width, including the scrollbar.
            self.width = self.rows
                .iter()
                .map(|row| row.width)
                .max()
                .map(|w| w + scrollbar);

            // Our resulting size.
            // We can't go lower, width-wise.

            let mut my_size = Vec2::new(self.width.unwrap_or(0),
                                        self.rows.len());

            if self.scrollable && my_size.y > size.y {
                my_size.y = size.y;
            }


            // println_stderr!("my: {:?} | si: {:?}", my_size, size);
            self.last_size = Some(SizeCache::build(my_size, size));
        }
    }

    // Invalidates the cache, so next call will recompute everything.
    fn invalidate(&mut self) {
        self.last_size = None;
    }
}


impl View for TextView {
    fn draw(&self, printer: &Printer) {

        let h = self.rows.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer =
            &printer.sub_printer(Vec2::new(0, offset), printer.size, true);

        self.scrollbase.draw(printer, |printer, i| {
            let row = &self.rows[i];
            let text = &self.content[row.start..row.end];
            let l = text.width();
            let x = self.align.h.get_offset(l, printer.size.x);
            printer.print((x, 0), text);
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.scrollbase.scrollable() {
            return EventResult::Ignored;
        }

        match event {
            Event::Key(Key::Home) => self.scrollbase.scroll_top(),
            Event::Key(Key::End) => self.scrollbase.scroll_bottom(),
            Event::Key(Key::Up) if self.scrollbase.can_scroll_up() => {
                self.scrollbase.scroll_up(1)
            }
            Event::Key(Key::Down) if self.scrollbase
                .can_scroll_down() => self.scrollbase.scroll_down(1),
            Event::Key(Key::PageDown) => self.scrollbase.scroll_down(10),
            Event::Key(Key::PageUp) => self.scrollbase.scroll_up(10),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }

    fn needs_relayout(&self) -> bool {
        self.last_size.is_none()
    }

    fn get_min_size(&mut self, size: Vec2) -> Vec2 {
        self.compute_rows(size);

        // This is what we'd like
        let mut ideal = Vec2::new(self.width.unwrap_or(0), self.rows.len());

        if self.scrollable && ideal.y > size.y {
            ideal.y = size.y;
        }

        ideal
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.scrollbase.scrollable()
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.
        self.compute_rows(size);
        self.scrollbase.set_heights(size.y, self.rows.len());
    }
}
