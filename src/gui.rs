use std::rc::Rc;

use git2::Repository;
use iced::advanced::{Widget, renderer, layout, widget, Layout};
use iced::widget::{text, Column, Row};
use iced::{Alignment, Element, Sandbox, Settings, Length, Rectangle, Color, mouse, Size};

use crate::backend::CommitNode;

pub struct GitUI {
    repository: Repository,
    commits: Vec<Rc<CommitNode>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RefreshTree,
}

impl GitUI {
    pub fn start() {
        Self::run(Settings::default()).unwrap()
    }
}

impl Sandbox for GitUI {
    type Message = Message;

    fn new() -> Self {
        let repository = match Repository::open(".") {
            Ok(repository) => repository,
            Err(e) => panic!("Error opening repository: {}", e),
        };

        let mut ui = Self { repository, commits: Vec::new() };
        ui.update(Message::RefreshTree);

        ui
    }

    fn title(&self) -> String {
        String::from("GitUI")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::RefreshTree => {
                let references = self.repository.references().unwrap();
                for reference in references {
                    let as_commit = reference.unwrap().peel_to_commit();

                    match as_commit.ok() {
                        Some(commit) => { CommitNode::create(commit, &mut self.commits); },
                        None => (),
                    }
                }

                for commit in &self.commits {
                    println!("{}", commit.as_ref().id);
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        Column::with_children({
            let mut children = Vec::new();

            children.push(Column::with_children({
                let mut children = Vec::new();

                children.push(Row::with_children({
                    vec![
                        text("Remotes").size(30).into()
                    ]
                })
                .align_items(Alignment::Center)
                .spacing(10)
                .into());

                //for remote in &self.remotes {
                //    children.push(text(remote).size(20).into());
                //}

                children
            }).into());

            children.push(Element::new(TreeRenderer));

            children
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }
}

struct TreeRenderer;

impl<Message, Renderer> Widget<Message, Renderer> for TreeRenderer
where Renderer: renderer::Renderer {
    fn width(&self) -> Length {
        Length::Fill
    }

    fn height(&self) -> Length {
        Length::Fill
    }

    fn layout(
        &self,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.width(Length::Fill).height(Length::Fill).resolve(Size::ZERO);
        layout::Node::new(Size::new(size.width, size.width))
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Renderer::Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle { x: viewport.width / 2.0 - 50.0, y: viewport.height / 2.0 - 50.0, width: 100.0, height: 100.0 },
                border_radius: 50.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Color::from_rgb(0.35, 0.35, 0.35),
        );
    }
}
