use std::rc::Rc;
use gtk::GtkWindowExt;
use gtk;
use glib;
use gtk::prelude::*;

use log::*;

use relm::{Relm, Component, Update, Widget, connect, connect_stream};
use relm::init as relm_init;
use relm_derive::Msg;

use notmuch::DatabaseMode;
use enamel_core::database::Thread;

use crate::app::EnamelApp;
use crate::headerbar::HeaderBar;

use crate::components::tag_list::{TagList, Msg as TagListMsg};
use crate::components::thread_list::{ThreadList, Msg as ThreadListMsg};
use crate::components::thread_view::{ThreadView, Msg as ThreadViewMsg};


#[derive(Msg)]
pub enum Msg {
    TagSelect(Option<String>),
    ThreadSelect(Thread),
    Change,
    Quit,
}

#[derive(Clone)]
pub struct Model {
    relm: Relm<MainWindow>,
    app: Rc<EnamelApp>
}

#[derive(Clone)]
struct Widgets {
    headerbar: Component<HeaderBar>,
    taglist: Component<TagList>,
    threadlist: Component<ThreadList>,
    threadview: Component<ThreadView>
}



// TODO: Factor out the hamburger menu
// TODO: Make a proper state machine for the headerbar states
pub struct MainWindow {
    model: Model,
    container: gtk::ApplicationWindow,
    widgets: Widgets
}

impl MainWindow {

    fn on_tag_changed(self: &mut Self, tag: Option<String>){

        // TODO: build a new query and refresh the thread list.
        let dbman = self.model.app.dbmanager.clone();
        let db = dbman.get(DatabaseMode::ReadOnly).unwrap();

        let qs = match tag{
            Some(tag) => format!("tag:{}", tag).to_string(),
            None => "".to_string()
        };
        debug!("qs: {:?}", qs);


        let query = <notmuch::Database as notmuch::DatabaseExt>::create_query(db, &qs).unwrap();
        let threads = <notmuch::Query<'_> as notmuch::QueryExt>::search_threads(query).unwrap();

        self.widgets.threadlist.emit(ThreadListMsg::Update(Some(threads)));
    }

    fn on_thread_selected(self: &mut Self, thread: Thread){
        self.widgets.threadview.emit(ThreadViewMsg::ShowThread(thread))
    }


}

impl Update for MainWindow{
    type Model = Model;
    type ModelParam = Rc<EnamelApp>;
    type Msg = Msg;

    fn model(relm: &Relm<Self>, app: Self::ModelParam) -> Model {
        Self::Model {
            relm: relm.clone(),
            app
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::TagSelect(tag) => self.on_tag_changed(tag),
            Msg::ThreadSelect(thread) => self.on_thread_selected(thread),
            Msg::Change => {
                // self.model.content = self.widgets.input.get_text()
                //                                        .expect("get_text failed")
                //                                        .chars()
                //                                        .rev()
                //                                        .collect();
                // self.widgets.label.set_text(&self.model.content);
            },
            Msg::Quit => gtk::main_quit(),
        }
    }
}

impl Widget for MainWindow {
    type Root = gtk::ApplicationWindow;

    fn root(&self) -> Self::Root {
        self.container.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        
        let window = model.app.builder.get_object::<gtk::ApplicationWindow>("main_window")
                                  .expect("Couldn't find main_window in ui file.");
        window.set_application(Some(&model.app.instance));


        let headerbar = relm_init::<HeaderBar>(model.app.clone()).unwrap(); 
        let taglist = relm_init::<TagList>(model.app.clone()).unwrap(); 
        let threadlist = relm_init::<ThreadList>(model.app.clone()).unwrap(); 
        let threadview = relm_init::<ThreadView>(model.app.clone()).unwrap(); 


        // TODO: what would be the best place to connect all UI signals?
        use self::TagListMsg::ItemSelect as TagList_ItemSelect;
        connect!(taglist@TagList_ItemSelect(ref tag), relm, Msg::TagSelect(tag.clone()));

        use self::ThreadListMsg::ThreadSelect as ThreadList_ThreadSelect;
        connect!(threadlist@ThreadList_ThreadSelect(ref thread), relm, Msg::ThreadSelect(thread.as_ref().unwrap().clone()));



        MainWindow {
            model,
            container: window,
            widgets: Widgets{
                headerbar,
                taglist,
                threadlist,
                threadview
            }
        }

    }

    fn init_view(&mut self) {

        let main_paned = self.model.app.builder.get_object::<gtk::Paned>("main_paned")
                                   .expect("Couldn't find main_paned in ui file.");

        let taglist_header = self.model.app.builder.get_object::<gtk::HeaderBar>("taglist_header")
                                 .expect("Couldn't find taglist_header in ui file.");

        // // TODO: do I need to unbind this at some point?
        // let _width_bind = main_paned.bind_property("position", &taglist_header, "width-request")
        //                             .flags(glib::BindingFlags::SYNC_CREATE)
        //                             .transform_to(move |_binding, value| {
        //                                 let offset = 6; //TODO: this offset was trial and error.
        //                                                 // we should calculate it somehow.
        //                                 Some((value.get::<i32>().unwrap_or(Some(0)) + offset).to_value())
        //                             })
        //                             .build();

        self.container.show_all();

        self.widgets.taglist.emit(TagListMsg::Refresh);
    }

}