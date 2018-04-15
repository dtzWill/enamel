use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use scoped_pool::Pool;

use gio;
use glib;
use glib::translate::FromGlib;
use gtk;
use gtk::prelude::*;
use relm;
use relm_attributes::widget;

use notmuch;
use notmuch::DatabaseMode;

use inox_core::settings::Settings;
use inox_core::database::Manager as DBManager;

use thread_list_item::ThreadListItem;

fn append_text_column(tree: &gtk::TreeView, id: i32) {
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();

    column.pack_start(&cell, true);
    // Association of the view's column with the model's `id` column.
    column.add_attribute(&cell, "text", id);
    tree.append_column(&column);
}


#[derive(Msg, Debug)]
pub enum Msg {
    // outbound
    ItemSelect,

    // inbound
    Update(String),

    // private
    AddThreads
}

pub struct ThreadList{
    model: ThreadListModel,
    scrolled_window: gtk::ScrolledWindow,
    tree_view: gtk::TreeView,
    tree_filter: gtk::TreeModelFilter,
    tree_model: gtk::ListStore
}


pub struct ThreadListModel {
    relm: ::relm::Relm<ThreadList>,
    settings: Rc<Settings>,
    dbmanager: Arc<DBManager>,
}


#[derive(Default, Debug)]
struct MailThread {
    pub id: String,
    pub subject: String,
    pub total_messages: i32,
    pub authors: Vec<String>,
    pub oldest_date: i64,
    pub newest_date: i64
}


fn add_thread(tree_model: gtk::ListStore, thread: MailThread){

    let subject = &thread.subject;
    let it = tree_model.append();
    tree_model.set_value(&it, 0, &thread.subject.to_value());

}

impl ThreadList{

    fn update(&mut self, qs: String){
        self.tree_model.clear();

        let mut dbman = self.model.dbmanager.clone();

        let db = dbman.get(DatabaseMode::ReadOnly).unwrap();

        let tree_model = self.tree_model.clone();

        let (tx, rx): (Sender<MailThread>, Receiver<MailThread>)  = channel();

        thread::spawn(move || {

            let query = db.create_query(&qs).unwrap();

            let mut threads = query.search_threads().unwrap();

            loop {
                match threads.next() {
                    Some(mthread) => {
                        // let thrd = Arc::new(RwLock::new(thread));
                        tx.send(MailThread{
                            id: mthread.id(),
                            subject: mthread.subject(),
                            total_messages: mthread.total_messages(),
                            authors: mthread.authors(),
                            oldest_date: mthread.oldest_date(),
                            newest_date: mthread.newest_date()

                        }).unwrap();
                    },
                    None => { break }
                }
            }

        });



        gtk::timeout_add(250, move || {
            loop {
                match rx.try_recv(){
                    Ok(thread) => {
                        add_thread(tree_model.clone(), thread);

                    },
                    Err(err) if err == TryRecvError::Empty => {
                        return Continue(true);
                    },
                    Err(err) => {
                        return Continue(false);
                    },
                }
            }
            Continue(false)
        });


    }

}


impl ::relm::Update for ThreadList {
    type Model = ThreadListModel;
    type ModelParam = (Rc<Settings>, Arc<DBManager>);
    type Msg = Msg;

    fn model(relm: &::relm::Relm<Self>, (settings, dbmanager): Self::ModelParam) -> Self::Model {
        ThreadListModel {
            relm: relm.clone(),
            settings,
            dbmanager
        }
    }

    fn update(&mut self, event: Self::Msg) {
        match event {
            Msg::Update(ref qs) => self.update(qs.clone()),
            Msg::ItemSelect => (),
            Msg::AddThreads => ()
        }
    }
}


impl ::relm::Widget for ThreadList {

    type Root = gtk::ScrolledWindow;

    fn root(&self) -> Self::Root {
        self.scrolled_window.clone()
    }

    fn view(relm: &::relm::Relm<Self>, model: Self::Model) -> Self
    {
        let scrolled_window = gtk::ScrolledWindow::new(None, None);

        let tree_model = gtk::ListStore::new(&[String::static_type()]);
        let tree_filter = gtk::TreeModelFilter::new(&tree_model, None);
        let tree_view = gtk::TreeView::new_with_model(&tree_filter);
        // let tree_view = gtk::TreeView::new();

        tree_view.set_headers_visible(false);
        append_text_column(&tree_view, 0);

        scrolled_window.add(&tree_view);

        connect!(relm, tree_view, connect_cursor_changed(_), Msg::ItemSelect);

        ThreadList {
            model,
            scrolled_window,
            tree_view,
            tree_filter,
            tree_model,
        }
    }
}
