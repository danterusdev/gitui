use git2::Repository;
use iced::widget::{button, text, Column, Row};
use iced::{Alignment, Element, Sandbox, Settings};

pub struct GitUI {
    repository: Repository,
    remotes: Vec<String>,
    changes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    RefreshRemotes,
    RefreshChanged,
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

        Self { repository, remotes: Vec::new(), changes: Vec::new() }
    }

    fn title(&self) -> String {
        String::from("GitUI")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::RefreshRemotes => {
                let remotes = self.repository.remotes().unwrap();
                self.remotes.clear();
                for remote in remotes.iter() {
                    self.remotes.push(String::from(remote.unwrap()));
                }
            },
            Message::RefreshChanged => {
                let changes = self.repository.diff_index_to_workdir(None, None).unwrap();
                self.changes.clear();
                for change in changes.deltas() {
                    self.remotes.push(String::from(change.new_file().path().unwrap().to_str().unwrap()));
                }
            },
        }
    }

    fn view(&self) -> Element<Message> {
        Column::with_children({
            let mut children = Vec::new();

            children.push(Row::with_children({
                let mut children = Vec::new();
                children.push(text("Remotes").size(30).into());
                children.push(button("Refresh").on_press(Message::RefreshRemotes).into());

                children
            })
            .spacing(10)
            .into());

            for remote in &self.remotes {
                children.push(text(remote).size(20).into());
            }

            children.push(Row::with_children({
                let mut children = Vec::new();
                children.push(text("Changed").size(30).into());
                children.push(button("Refresh").on_press(Message::RefreshChanged).into());

                children
            })
            .spacing(10)
            .into());

            for changes in &self.changes {
                children.push(text(changes).size(20).into());
            }

            children
        })
        .padding(20)
        .align_items(Alignment::Center)
        .into()
    }
}
