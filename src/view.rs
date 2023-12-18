use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use js_sys::{Function, Reflect};
use wasm_bindgen::prelude::{wasm_bindgen, Closure};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Element, HtmlElement, Node, Text};

use super::dynamic::TreeSubscriber;
use super::item::TreeItem;
use super::KeyType;

pub trait TreeController {
    fn item(&self, index: usize) -> Rc<dyn TreeItem>;
    fn count(&self) -> usize;
    fn handle_click(&self, key: usize);
    fn add_subscriber(&self, subscriber: Rc<dyn TreeSubscriber>);
}

pub struct TreeState {
    count: usize,
    rendered: HashMap<KeyType, RenderedItem>,
    pool: Vec<RenderedItem>,
    offset: usize,
    size: (usize, usize),
}

pub struct TreeView {
    state: RefCell<TreeState>,

    pub container: HtmlElement,
    pub scroll: HtmlElement,
    item_height: usize,

    ctrl: Rc<dyn TreeController>,

    #[allow(unused)]
    on_resize: Closure<dyn Fn(JsValue)>,
    #[allow(unused)]
    on_click: Closure<dyn Fn(JsValue)>,
    #[allow(unused)]
    on_scroll: Closure<dyn Fn(JsValue)>,
    #[allow(unused)]
    observer: JsValue,
}

impl Drop for TreeView {
    fn drop(&mut self) {
        self.container.remove();
        unobserve(&self.observer)
    }
}

impl TreeView {
    pub fn render(handle: Rc<dyn TreeController>) -> Rc<TreeView> {
        let tree = Rc::<TreeView>::new_cyclic(|this| {
            let document = web_sys::window().unwrap().document().unwrap();
            let container: HtmlElement = document.create_element("div").unwrap().unchecked_into();
            container.set_attribute("class", "tree").unwrap();

            let scroll: HtmlElement = document.create_element("div").unwrap().unchecked_into();
            scroll.set_attribute("class", "tree-scroll").unwrap();

            container.append_child(&scroll).unwrap();

            let on_click: Closure<dyn Fn(JsValue)> = Closure::new({
                let this = this.clone();
                move |ev: JsValue| {
                    this.upgrade().unwrap().handle_click(ev.unchecked_into());
                }
            });

            container
                .add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())
                .unwrap();

            let on_scroll: Closure<dyn Fn(JsValue)> = Closure::new({
                let this = this.clone();
                move |_: JsValue| {
                    let this = this.upgrade().unwrap();
                    let offset = this.container.scroll_top();
                    this.update_scroll(offset as usize);
                }
            });

            container
                .add_event_listener_with_callback("scroll", on_scroll.as_ref().unchecked_ref())
                .unwrap();

            let on_resize: Closure<dyn Fn(JsValue)> = Closure::new({
                let this = this.clone();
                move |size: JsValue| {
                    let width = Reflect::get_u32(&size, 0).unwrap().as_f64().unwrap() as usize;
                    let height = Reflect::get_u32(&size, 1).unwrap().as_f64().unwrap() as usize;
                    this.upgrade().unwrap().update_size(width, height);
                }
            });

            let observer = observe(&container, on_resize.as_ref().unchecked_ref());
            let size = (0, 0);
            let offset = 0;
            let item_height = 24;
            let count = handle.count();

            let tree = TreeView {
                ctrl: handle,
                state: RefCell::new(TreeState {
                    count,
                    size,
                    offset,
                    rendered: Default::default(),
                    pool: vec![],
                }),
                scroll,
                container,
                observer,
                on_resize,
                on_scroll,
                on_click,
                item_height,
            };

            tree.update();

            tree
        });

        tree.ctrl.add_subscriber(tree.clone());

        tree
    }

    fn handle_click(&self, ev: web_sys::MouseEvent) {
        tracing::info!("Handle click event");

        let target = ev.target().unwrap().unchecked_into::<HtmlElement>();
        if let Some(item) = target.closest("[data-key]").unwrap() {
            let key = item.get_attribute("data-key").unwrap();
            let key = key.parse::<usize>().unwrap();
            tracing::info!("Handle click event for key {}", key);

            ev.prevent_default();
            ev.stop_propagation();
            self.ctrl.handle_click(key);
        }
    }

    pub fn update_size(&self, width: usize, height: usize) {
        self.state.borrow_mut().size = (width, height);
        self.update();
    }

    pub fn update_scroll(&self, offset_top: usize) {
        self.state.borrow_mut().offset = offset_top;
        self.update();
    }

    #[inline]
    pub fn calc_shift(&self, item: &dyn TreeItem) -> usize {
        // TODO: Make those offsets customizable
        let mut offset = (item.depth()) as usize * 16 + 10;
        if item.expandable() {
            offset -= 16
        }

        offset
    }

    pub fn update(&self) {
        const LABEL: &'static str = "Tree::update";
        web_sys::console::time_with_label(LABEL);

        if let Ok(value) = Reflect::get(&web_sys::window().unwrap(), &"__debug".into()) {
            if value.is_truthy() {
                panic!("__debug")
            }
        }

        let mut state = self.state.borrow_mut();
        // for splitting borrows
        let state = &mut *state;

        let count = self.ctrl.count();
        let offset = state.offset;
        let size = state.size;

        state.count = count;

        // Update can happen because of:
        //  scroll
        //  resize
        //  change items [diff can work]

        let first_visible = offset / self.item_height;
        let visible_count = size.1 / self.item_height + 2;

        self.scroll
            .set_attribute(
                "style",
                &format!("height: {}px", self.item_height * state.count),
            )
            .unwrap();

        let rendered = &mut state.rendered;

        let right = state.count.min(first_visible + visible_count);
        let range = first_visible..right;

        let mut visited = BTreeSet::<KeyType>::new();

        for (i, index) in range.enumerate() {
            let item = self.ctrl.item(index);
            let key = item.key();

            visited.insert(key);

            let y = (first_visible + i) * self.item_height;
            if let Some(rendered) = rendered.get_mut(&key) {
                rendered.update_style(y, self.calc_shift(&*item));
                rendered.update_item(&*item);
            } else {
                let rendered_item = if let Some(mut rendered) = state.pool.pop() {
                    rendered.update_item(&*item);
                    rendered.update_style(y, self.calc_shift(&*item));
                    rendered
                } else {
                    RenderedItem::render(&*item, y, self.calc_shift(&*item))
                };

                self.scroll.append_child(&rendered_item.container).unwrap();
                rendered.insert(item.key(), rendered_item);
            }
        }

        for (_key, item) in rendered.extract_if(|key, _| !visited.contains(&key)) {
            // pool
            item.container.remove();
            state.pool.push(item)
        }

        web_sys::console::time_end_with_label(LABEL);
    }
}

impl TreeSubscriber for TreeView {
    fn update_all(&self) {
        TreeView::update(self)
    }

    fn update_item(&self, _key: usize) {
        // TODO: More efficient selective update
        TreeView::update(self)
    }
}

pub struct RenderedItem {
    container: Element,
    hash: u64,
    expandable: bool,
    expanded: bool,
    arrow: Element,
    icon: Element,
    text: Text,
}

impl RenderedItem {
    pub fn render(item: &dyn TreeItem, y: usize, x: usize) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();

        let container = document.create_element("div").unwrap();
        container.set_attribute("class", "tree-item").unwrap();
        container
            .set_attribute("data-key", &item.key().to_string())
            .unwrap();

        let expanded = item.expanded();
        let expandable = item.expandable();

        let arrow = document.create_element("span").unwrap();
        if !expandable {
            arrow.set_attribute("style", "display: none").unwrap();
        }

        arrow
            .set_attribute("class", Self::expanded_classname(expanded))
            .unwrap();

        container.append_child(&arrow).unwrap();

        let icon = document.create_element("span").unwrap();
        let icon_class = item.icon();
        if !icon_class.is_empty() {
            icon.set_attribute("class", &icon_class).unwrap();
        }

        container.append_child(&icon).unwrap();

        let text_span = document.create_element("span").unwrap();
        let text = document.create_text_node(&item.title());
        text_span.append_child(&text).unwrap();
        container.append_child(&text_span).unwrap();

        let hash = item.hash();
        let mut this = Self {
            container,
            expandable,
            arrow,
            expanded,
            icon,
            text,
            hash,
        };

        this.update_style(y, x);

        this
    }

    fn expanded_classname(expanded: bool) -> &'static str {
        if expanded {
            "iconoir-nav-arrow-down"
        } else {
            "iconoir-nav-arrow-right"
        }
    }

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        let classname = Self::expanded_classname(self.expanded);
        self.arrow.set_attribute("class", classname).unwrap();
    }

    pub fn update_style(&mut self, y: usize, x: usize) {
        // FIXME: bump-allocate this
        let style = format!("top: {}px; padding-left: {}px", y, x);
        self.container.set_attribute("style", &style).unwrap();
    }

    pub fn update_item(&mut self, item: &dyn TreeItem) {
        if self.expandable != item.expandable() {
            self.expandable = item.expandable();
            if self.expandable {
                self.arrow.remove_attribute("style").unwrap();
            } else {
                self.arrow.set_attribute("style", "display: none").unwrap();
            }
        }

        if self.expanded != item.expanded() {
            self.toggle_expanded()
        }

        let hash = item.hash();
        if hash == self.hash {
            return;
        }

        self.hash = hash;

        self.container
            .set_attribute("data-key", &item.key().to_string())
            .unwrap();

        self.icon.set_attribute("class", &*item.icon()).unwrap();

        self.text.set_data(&item.title());
    }
}

#[wasm_bindgen]
extern "C" {
    fn observe(element: &Node, callback: &Function) -> JsValue;
    fn unobserve(ro: &JsValue);
}
