// examples.rs ----------------------------------------------------------------------------------------------------

use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::Xplr;
use crate::fenst::example::{XplrExample, XplrExampleResult};
use crate::silo::{Arr, Buff, Stash};
use iced::widget::{button, column, container, mouse_area, text};
use iced::{Element, Length, Task};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

//-------------------------------------------------------------------------------------------------

/// A cloneable, single-consumption delivery envelope for an owned result tree.
#[derive(Clone)]
pub struct ExampleCompletion(Arc<Mutex<Option<XplrExampleResult>>>);

impl std::fmt::Debug for ExampleCompletion
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        return formatter.write_str("ExampleCompletion");
    }
}

impl ExampleCompletion
{
    pub fn New(result: XplrExampleResult) -> Self
    {
        return Self(Arc::new(Mutex::new(Some(result))));
    }
}

#[derive(Debug, Clone)]
pub enum ExampleAction
{
    Select(u32),
    Run(u32),
    Completed(u32, ExampleCompletion),
    Toggle(u32),
}

struct ExampleNode
{
    _Owner:    Rc<Buff<Box<dyn Xplr>>>,
    _Index:    u32,
    _Children: Option<Buff<u32>>,
    _Expanded: bool,
}

impl ExampleNode
{
    fn Xplr(&self) -> &dyn Xplr { return self._Owner[self._Index].as_ref(); }
}

struct ExampleRoot
{
    _Title: String,
    _Node:  u32,
}

/// Owns all returned Xplr nodes; paths are labels, never filesystem lookup keys.
pub struct ExampleBrowser
{
    _Examples: Buff<&'static XplrExample>,
    _Roots:    Stash<ExampleRoot>,
    _Nodes:    Stash<ExampleNode>,
    _Selected: u32,
    _Running:  u32,
    _Status:   String,
    _Preview:  String,
}

impl Default for ExampleBrowser
{
    fn default() -> Self
    {
        return Self { _Examples: XplrExample::Registered(),
                      _Roots:    Stash::New(),
                      _Nodes:    Stash::New(),
                      _Selected: u32::MAX,
                      _Running:  u32::MAX,
                      _Status:   "Double-click an example to run".into(),
                      _Preview:  String::new(), };
    }
}

impl ExampleBrowser
{
    pub fn Examples(&self) -> Arr<'_, &'static XplrExample> { return self._Examples.Arr(); }

    pub fn ResultCount(&self) -> u32 { return self._Roots.Size(); }

    pub fn ResultNode(&self, index: u32) -> u32 { return self._Roots[index]._Node; }

    pub fn Children(&self, node: u32) -> Arr<'_, u32>
    {
        return self._Nodes[node]._Children
                                .as_ref()
                                .map(Buff::Arr)
                                .unwrap_or_default();
    }

    pub fn Status(&self) -> &str { return &self._Status; }

    pub fn Preview(&self) -> &str { return &self._Preview; }

    pub fn Update(&mut self, action: ExampleAction) -> Task<ExampleAction>
    {
        match action {
            ExampleAction::Select(index) => self._Selected = index,
            ExampleAction::Run(index) => {
                if self._Running != u32::MAX || index >= self._Examples.Len() {
                    return Task::none();
                }
                self._Selected = index;
                self._Running = index;
                let example = self._Examples[index];
                self._Status = format!("Running {}::{}...", example.suite, example.name);
                return Task::perform(Execute(example), move |result| {
                    ExampleAction::Completed(index, result)
                });
            }
            ExampleAction::Completed(index, completion) => {
                if self._Running != index || index >= self._Examples.Len() {
                    return Task::none();
                }
                let Some(result) = completion.0.lock().unwrap().take() else {
                    return Task::none();
                };
                self._Running = u32::MAX;
                let example = self._Examples[index];
                match result {
                    Ok(root) => {
                        let node = self._Nodes.Size();
                        let root: Box<dyn Xplr> = root;
                        self._Nodes
                            .Push(ExampleNode { _Owner:    Rc::new(crate::Buff![root]),
                                                _Index:    0,
                                                _Children: None,
                                                _Expanded: false, });
                        self._Roots.Push(ExampleRoot { _Title: format!("{}::{} #{}",
                                                                       example.suite,
                                                                       example.name,
                                                                       self._Roots.Size() + 1),
                                                       _Node:  node, });
                        self._Status = format!("Passed {}::{}", example.suite, example.name);
                    }
                    Err(error) => {
                        self._Status =
                            format!("Failed {}::{}: {error}", example.suite, example.name)
                    }
                }
            }
            ExampleAction::Toggle(index) => self.Inspect(index),
        }
        return Task::none();
    }

    fn Inspect(&mut self, index: u32)
    {
        if index >= self._Nodes.Size() {
            return;
        }
        let node = &self._Nodes[index];
        let source = node.Xplr();
        self._Preview = format!("{}\n{}", source.Name(), source.Path());
        if let Some(leaf) = source.Leaf() {
            self._Preview = match leaf.ReadChunk(0, 4096) {
                Ok(chunk) => {
                    format!("{}\n{} bytes\n{}{}",
                            source.Name(),
                            leaf.Size(),
                            chunk.Content(),
                            if chunk.IsEof() {
                                ""
                            } else {
                                "\n[Preview limited to 4096 bytes]"
                            })
                }
                Err(error) => format!("Cannot read {}: {error}", source.Name()),
            };
            return;
        }
        if node._Expanded {
            self._Nodes[index]._Expanded = false;
            return;
        }
        if node._Children.is_none() {
            let Some(branch) = source.Branch() else {
                return;
            };
            match branch.Children() {
                Ok(children) => {
                    let owner = Rc::new(children);
                    let ids = Buff::FromDispenser(owner.Len(), |child| {
                        let id = self._Nodes.Size();
                        self._Nodes.Push(ExampleNode { _Owner:    owner.clone(),
                                                       _Index:    child,
                                                       _Children: None,
                                                       _Expanded: false, });
                        return id;
                    });
                    self._Nodes[index]._Children = Some(ids);
                }
                Err(error) => {
                    self._Preview = format!("Cannot expand {}: {error}", source.Name());
                    return;
                }
            }
        }
        self._Nodes[index]._Expanded = true;
    }

    pub fn View(&self, palette: ThemePalette) -> Element<'_, ExampleAction>
    {
        let mut items = column![text("EXAMPLES").size(11), text(self.Status()).size(11)].spacing(4);
        let mut suite = "";
        self._Examples.USeg().Traverse( |index| {
            let example = self._Examples[index];
            if suite != example.suite
            {
                suite = example.suite;
                items = std::mem::replace( &mut items, column![]).push( text( suite).size( 12));
            }
            let selected = self._Selected == index;
            // The mouse area owns both click events; a nested button would consume them.
            let entry = container( text( example.name).size( 12))
                .padding( [4, 10]).width( Length::Fill)
                .style( move |_| container::Style {
                    background: selected.then_some( iced::Background::Color( palette.accent)),
                    ..Default::default()
                });
            let mut area = mouse_area( entry).on_press( ExampleAction::Select( index));
            if self._Running == u32::MAX
            {
                area = area.on_double_click( ExampleAction::Run( index));
            }
            items = std::mem::replace( &mut items, column![]).push( area);
        });
        if self._Examples.IsEmpty() {
            items = items.push(text("No Xplr examples registered in this build.").size(12));
        }
        if !self._Roots.IsEmpty() {
            items = items.push(text("RESULTS").size(11));
            self._Roots.USeg().Traverse( |rootIndex| {
                let root = &self._Roots[rootIndex];
                items = std::mem::replace( &mut items, column![]).push( text( &root._Title).size( 11));
                let mut pending = Stash::New();
                pending.Push( (root._Node, 0u32));
                while let Some( (index, depth)) = pending.Pop()
                {
                    let node = &self._Nodes[index];
                    let arrow = if node.Xplr().Branch().is_some()
                    {
                        if node._Expanded { "v" } else { ">" }
                    }
                    else { "-" };
                    let entry = button( text( format!( "{arrow} {}", node.Xplr().Name())).size( 12))
                        .padding( [3, 6]).width( Length::Fill)
                        .style( move |_, status| FasciaStyle::tree_row_button( palette, false, status))
                        .on_press( ExampleAction::Toggle( index));
                    items = std::mem::replace( &mut items, column![]).push( container( entry)
                        .padding( iced::Padding::default().left( depth.min( 16) as f32 * 10.0)));
                    if node._Expanded && let Some( children) = &node._Children
                    {
                        children.USeg().TraverseRev( |child| pending.Push( (children[child], depth + 1)));
                    }
                }
            });
        }
        if !self._Preview.is_empty() {
            items = items.push(text(self.Preview()).size(12));
        }
        return container(items).padding(8).width(Length::Fill).into();
    }
}

//-------------------------------------------------------------------------------------------------

async fn Execute(example: &'static XplrExample) -> ExampleCompletion
{
    let (reply, receiver) = iced::futures::channel::oneshot::channel();
    let worker = std::thread::Builder::new().name("xplr-example".into())
                                            .spawn(move || {
                                                let atelier = crate::heist::Atelier::New(1);
                                                atelier.MainMaestro().Post( move |_| {
            let _guard = crate::cove::context::StandardTestGuard();
            let _ = reply.send( example.Run());
        });
                                                atelier.DoLaunch();
                                            });
    if let Err(error) = worker {
        return ExampleCompletion::New(Err(error.to_string()));
    }
    return ExampleCompletion::New(receiver.await.unwrap_or_else(|_| {
                                                    Err("Example worker stopped".into())
                                                }));
}

//-------------------------------------------------------------------------------------------------
