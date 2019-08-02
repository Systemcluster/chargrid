extern crate prototty;
use prototty::decorator::*;
use prototty::input::*;
use prototty::input::{keys, Input, KeyboardInput};
use prototty::render::*;
use prototty::text::*;

pub enum ControlFlow {
    Exit,
}

pub struct AppState {
    scroll_state: VerticalScrollState,
    text: String,
    border_style: BorderStyle,
    bound: Size,
    background: Rgb24,
    alignment: Alignment,
    scrollbar: VerticalScrollbar,
}

pub struct AppView {
    view: AlignView<FillBackgroundView<BorderView<BoundView<VerticalScrollView<RichTextView<wrap::Word>>>>>>,
}

impl AppView {
    pub fn new() -> Self {
        Self {
            view: AlignView::new(FillBackgroundView::new(BorderView::new(BoundView::new(
                VerticalScrollView::new(RichTextView::new(wrap::Word::new())),
            )))),
        }
    }
    fn scroll(&self) -> &VerticalScrollView<RichTextView<wrap::Word>> {
        &self.view.view.view.view.view
    }
}

impl AppState {
    pub fn new(text: String) -> Self {
        Self {
            scroll_state: VerticalScrollState::new(),
            text,
            border_style: BorderStyle {
                title_style: Style {
                    bold: Some(true),
                    foreground: Some(Rgb24::new(0, 255, 0)),
                    background: Some(Rgb24::new(0, 64, 0)),
                    ..Default::default()
                },
                padding: BorderPadding {
                    right: 0,
                    left: 2,
                    top: 1,
                    bottom: 1,
                },
                ..BorderStyle::default_with_title("Pager")
            },
            bound: Size::new(40, 30),
            background: Rgb24::new(80, 80, 0),
            alignment: Alignment::centre(),
            scrollbar: VerticalScrollbar::default(),
        }
    }
    pub fn tick<I>(&mut self, inputs: I, view: &AppView) -> Option<ControlFlow>
    where
        I: IntoIterator<Item = Input>,
    {
        for input in inputs {
            match input {
                Input::Keyboard(keys::ETX)
                | Input::Keyboard(keys::ESCAPE)
                | Input::Keyboard(KeyboardInput::Char('q')) => {
                    return Some(ControlFlow::Exit);
                }
                Input::Mouse(MouseInput::MouseScroll { direction, .. }) => match direction {
                    ScrollDirection::Up => self.scroll_state.scroll_up_line(view.scroll()),
                    ScrollDirection::Down => self.scroll_state.scroll_down_line(view.scroll()),
                    _ => (),
                },
                Input::Keyboard(KeyboardInput::Up) => self.scroll_state.scroll_up_line(view.scroll()),
                Input::Keyboard(KeyboardInput::Down) => self.scroll_state.scroll_down_line(view.scroll()),
                Input::Keyboard(KeyboardInput::PageUp) => self.scroll_state.scroll_up_page(view.scroll()),
                Input::Keyboard(KeyboardInput::PageDown) => self.scroll_state.scroll_down_page(view.scroll()),
                Input::Keyboard(KeyboardInput::Home) | Input::Keyboard(KeyboardInput::Char('g')) => {
                    self.scroll_state.scroll_to_top(view.scroll())
                }
                Input::Keyboard(KeyboardInput::End) | Input::Keyboard(KeyboardInput::Char('G')) => {
                    self.scroll_state.scroll_to_bottom(view.scroll())
                }
                _ => (),
            }
        }
        None
    }
}

impl<'a> View<&'a AppState> for AppView {
    fn view<F: Frame, C: ColModify>(&mut self, app_state: &'a AppState, context: ViewContext<C>, frame: &mut F) {
        let rich_text = &[
            ("Hello, World!\nblah\nblah blah ", Style::default()),
            (
                "blue\n",
                Style {
                    foreground: Some(Rgb24::new(0, 0, 255)),
                    bold: Some(true),
                    ..Default::default()
                },
            ),
            ("User string:\n", Default::default()),
            (
                app_state.text.as_ref(),
                Style {
                    background: Some(Rgb24::new(187, 0, 0)),
                    underline: Some(true),
                    ..Default::default()
                },
            ),
        ];
        let data = AlignData {
            alignment: app_state.alignment,
            data: FillBackgroundData {
                background: app_state.background,
                data: BorderData {
                    style: &app_state.border_style,
                    data: BoundData {
                        size: app_state.bound,
                        data: VerticalScrollWithScrollbarData {
                            state: app_state.scroll_state,
                            scrollbar: app_state.scrollbar,
                            data: rich_text,
                        },
                    },
                },
            },
        };
        self.view.view(data, context, frame);
    }
}
