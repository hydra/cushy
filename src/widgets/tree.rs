use std::fmt::{Debug, Formatter};
use cushy::figures::units::Px;
use cushy::widget::{MakeWidget, WidgetRef, WrapperWidget};
use cushy::widgets::Space;
use indexmap::IndexMap;
use crate::reactive::value::{Destination, Dynamic, Source, Switchable};
use crate::widget::{IntoWidgetList, MakeWidgetList, WidgetInstance, WidgetList};
use crate::widgets::label::Displayable;

#[derive(Default,Clone, Debug, Hash, PartialEq, Eq)]
pub struct TreeNodeKey(usize);

pub struct TreeNode {
    parent: Option<TreeNodeKey>,
    depth: usize,
    child_widget: WidgetInstance,
    children: Dynamic<WidgetList>,
}

pub struct TreeNodeWidget {
    is_expanded: Dynamic<bool>,
    child: WidgetRef,
    child_height: Option<Px>,
}

impl TreeNodeWidget {
    pub fn new(child: WidgetInstance, children: Dynamic<WidgetList>) -> Self {

        let is_expanded = Dynamic::new(true);

        let indicator = is_expanded.clone().map_each(|value|{
            match value {
                true => "v",
                false => ">"
            }
        }).into_label();

        let expand_button = indicator.into_button()
            .on_click({
                let is_expanded = is_expanded.clone();
                move |_event| {
                    is_expanded.toggle();
                }
            })
            .make_widget();

        let children_switcher = is_expanded.clone().switcher(move |value, active| {
            match value {
                false => Space::default().make_widget(),
                true => children.clone().into_rows().make_widget()
            }
        }).make_widget();

        let child = expand_button
            .and(child)
            .into_columns()
            .and(children_switcher)
            .into_rows()
            // FIXME remove container, just for tree right now.
            .contain()
            .into_ref();

        Self {
            is_expanded,
            child,
            child_height: None,
        }
    }
}

impl Debug for TreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeNode")
            .field("parent", &self.parent)
            .field("depth", &self.depth)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Tree {
    nodes: Dynamic<IndexMap<TreeNodeKey, TreeNode>>,
    next_key: TreeNodeKey,
}

pub struct TreeWidget {
    root: WidgetRef,
}

impl Default for Tree {
    fn default() -> Self {
        let nodes = Dynamic::new(IndexMap::<TreeNodeKey, TreeNode>::new());

        Self {
            nodes,
            next_key: TreeNodeKey::default(),
        }
    }
}
impl Tree {
    pub fn make_widget(&self) -> WidgetInstance {
        let root = self.nodes.clone().switcher(|nodes, _active| {
            if nodes.is_empty()  {
                Space::default().make_widget()
            } else {
                let (_root_key, root_node) = nodes.first().unwrap();

                root_node.child_widget.clone()
            }
        }).into_ref();

        TreeWidget {
            root
        }.make_widget()
    }

    fn generate_next_key(&mut self) -> TreeNodeKey {
        let key = self.next_key.clone();
        self.next_key.0 += 1;
        key
    }

    /// Inserts a child after the given parent
    pub fn insert_child(&mut self, value: WidgetInstance, parent: Option<&TreeNodeKey>) -> Option<TreeNodeKey> {
        self.insert_child_f(|key|value, parent)
    }

    pub fn insert_child_f<F>(&mut self, value_f: F, parent: Option<&TreeNodeKey>) -> Option<TreeNodeKey>
    where
        F: FnOnce(TreeNodeKey) -> WidgetInstance
    {
        // Determine whether a new key and node should be created
        let (depth, parent_key) = {
            let nodes = self.nodes.lock();
            if let Some(parent) = parent {
                if let Some(parent_node) = nodes.get(parent) {
                    (Some(parent_node.depth + 1), Some(parent.clone()))
                } else {
                    (None, None) // Parent not found, node won't be inserted
                }
            } else {
                // If no parent is specified, this is a root node
                (Some(0), None)
            }
        };

        // If depth is determined, generate key and create the node
        if let Some(depth) = depth {
            let key = self.generate_next_key(); // Generate key after deciding a node is needed

            let value = value_f(key.clone());

            let children = Dynamic::new(WidgetList::new());
            let child_widget = TreeNodeWidget::new(value, children.clone()).make_widget();

            let child_node = TreeNode {
                parent: parent_key.clone(),
                depth,
                child_widget,
                children,
            };

            {
                let mut nodes = self.nodes.lock();
                nodes.insert(key.clone(), child_node);
            }

            self.update_children_widgetlist(parent_key);

            Some(key)
        } else {
            None
        }
    }

    fn update_children_widgetlist(&mut self, parent_key: Option<TreeNodeKey>) {
        if let Some(parent_key) = parent_key {
            // regenerate the 'children' widget list for the parent

            let children: WidgetList = self.children_keys(parent_key.clone())
                .into_iter()
                .enumerate()
                .map(|(index, key)| {
                    let nodes = self.nodes.lock();
                    let node = nodes.get(&key).unwrap();

                    index.into_label().make_widget()
                        .and(node.child_widget.clone())
                        .into_columns()
                        .make_widget()
                })
                .collect();

            let mut nodes = self.nodes.lock();
            let parent = nodes.get(&parent_key).unwrap();
            parent.children.set(children);
        }
    }

    /// Inserts a sibling after the given node.
    ///
    /// Returns `None` if the given node doesn't exist or is the root node.
    pub fn insert_after(&mut self, value: WidgetInstance, sibling: &TreeNodeKey) -> Option<TreeNodeKey> {
        self.insert_after_f(|key|value, sibling)
    }
    pub fn insert_after_f<F>(&mut self, value_f: F, sibling: &TreeNodeKey) -> Option<TreeNodeKey>
    where
        F: FnOnce(TreeNodeKey) -> WidgetInstance
    {

        // FIXME likely the API could be better, so that there is no concept of a 'root' node at all, then this limitation can be removed
        // cannot add siblings to the root, silently ignore.
        if self.nodes.lock().get(sibling).unwrap().parent.is_none() {
            return None
        }

        // Determine whether a new key and node should be created
        let (depth, parent_key) = {
            let nodes = self.nodes.lock();
            nodes.get(sibling).map_or((None, None), |node|{
                (Some(node.depth), Some(node.parent.clone().unwrap()))
            })
        };

        // If depth is determined, generate key and create the node
        if let Some(depth) = depth {
            let key = self.generate_next_key(); // Generate key after deciding a node is needed
            let value = value_f(key.clone());

            let children = Dynamic::new(WidgetList::new());
            let child_widget = TreeNodeWidget::new(value, children.clone()).make_widget();

            let child_node = TreeNode {
                parent: parent_key.clone(),
                depth,
                child_widget,
                children
            };

            {
                let mut nodes = self.nodes.lock();
                nodes.insert(key.clone(), child_node);
            }

            self.update_children_widgetlist(parent_key);

            Some(key)
        } else {
            None
        }
    }

    /// Clears the tree, removing all nodes and resetting the key.
    pub fn clear(&mut self) {
        self.nodes.lock().clear();
        self.next_key = TreeNodeKey::default();
    }

    /// Removes the node and all descendants.
    pub fn remove_node(&mut self, node_key: &TreeNodeKey) {
        let mut nodes = self.nodes.lock();
        // First, check if the node exists
        if !nodes.contains_key(node_key) {
            return;
        }

        // Create a stack to hold nodes to be removed
        let mut to_remove = vec![node_key.clone()];

        // We perform a DFS traversal to collect all descendant keys
        while let Some(current_key) = to_remove.pop() {
            if let Some(_node) = nodes.shift_remove(&current_key) {
                // Add children of the current node to the stack
                nodes
                    .keys()
                    .filter(|&key| nodes[key].parent.as_ref() == Some(&current_key))
                    .for_each(|key| to_remove.push(key.clone()));
            }
        }
    }

    pub fn children_keys(&self, parent_key: TreeNodeKey) -> Vec<TreeNodeKey> {
        let nodes = self.nodes.lock();
        nodes.iter()
            .filter_map(|(key, node)| {
                if node.parent.as_ref() == Some(&parent_key) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::widget::MakeWidget;
    use crate::widgets::label::Displayable;
    use super::Tree;
    
    #[test]
    pub fn add_root() {
        // given
        
        let mut tree = Tree::default();
        let root_widget = "root".into_label().make_widget();
        // when
        
        let key = tree.insert_child(root_widget, None).unwrap();

        // then
        let nodes = tree.nodes.lock();

        assert_eq!(key.0, 0);
        assert_eq!(nodes.len(), 1);
        // and
        let root = nodes.get(&key).unwrap();
        
        assert_eq!(root.parent, None);
        assert_eq!(root.depth, 0);
    }
    
    #[test]
    pub fn add_child_to_root() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();

        // when
        let child_key = tree.insert_child("child".to_string(), Some(&root_key)).unwrap();

        // then
        let nodes = tree.nodes.lock();

        assert_eq!(child_key.0, 1);
        assert_eq!(nodes.len(), 2);

        // and
        let child = nodes.get(&child_key).unwrap();
        assert_eq!(child.parent, Some(root_key.clone()));
        assert_eq!(child.depth, 1);
    }


    #[test]
    pub fn add_sibling_to_child() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        let first_child_key = tree.insert_child("first_child".to_string(), Some(&root_key)).unwrap();

        // when
        let sibling_key = tree.insert_after("sibling".to_string(), &first_child_key).unwrap();

        // then
        let nodes = tree.nodes.lock();
        assert_eq!(nodes.len(), 3);

        // and verify the sibling properties
        let sibling = nodes.get(&sibling_key).unwrap();
        assert_eq!(sibling.parent, Some(root_key.clone()));
        assert_eq!(sibling.depth, 1); // Assuming sibling has the same depth as the first child
    }


    #[test]
    pub fn remove_node() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        let child_key = tree.insert_child("child".to_string(), Some(&root_key)).unwrap();
        let _descendant_key = tree.insert_child("descendant".to_string(), Some(&child_key)).unwrap();

        // node to be removed
        let node_to_remove = root_key.clone();

        // assume we have a remove_node method
        tree.remove_node(&node_to_remove);

        // then
        let nodes = tree.nodes.lock();
        nodes.iter().for_each(|(key, node)| {
            println!("key: {:?}: node: {:?}", key, node);
        });
        // and root, child and descendant nodes should be removed
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    pub fn remove_child_node() {
        // given
        
        // Root
        // +- 1
        // |  +- 3
        // +- 2
        // |  +- 4
        
        
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        // direct children
        let key_1 = tree.insert_child("1".to_string(), Some(&root_key)).unwrap();
        let key_2 = tree.insert_child("2".to_string(), Some(&root_key)).unwrap();
        // descendants
        let key_3 = tree.insert_child("3".to_string(), Some(&key_1)).unwrap();
        let key_4 = tree.insert_child("3".to_string(), Some(&key_2)).unwrap();

        // ensure they exist before removal
        {
            let nodes = tree.nodes.lock();
            assert_eq!(nodes.len(), 5);
        }
        
        // node to be removed
        let node_to_remove = key_1.clone();

        // when
        tree.remove_node(&node_to_remove);

        // then the root node should remain
        let nodes = tree.nodes.lock();

        assert_eq!(nodes.len(), 3);
        assert!(nodes.get(&root_key).is_some());

        // other elements should remain
        assert!(nodes.get(&key_2).is_some());
        assert!(nodes.get(&key_4).is_some());

        // and child and children should be removed
        assert!(nodes.get(&key_1).is_none());
        assert!(nodes.get(&key_3).is_none());
    }

    #[test]
    pub fn children_keys() {
        // given
        let mut tree = Tree::default();
        let root_key = tree.insert_child("root".to_string(), None).unwrap();
        let child_key_1 = tree.insert_child("child_1".to_string(), Some(&root_key)).unwrap();
        let child_key_2 = tree.insert_child("child_2".to_string(), Some(&root_key)).unwrap();

        // when
        let children = tree.children_keys(root_key.clone());

        // then
        assert_eq!(children.len(), 2);
        assert!(children.contains(&child_key_1));
        assert!(children.contains(&child_key_2));
    }
}

impl Debug for TreeNodeWidget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeNodeWidget")
            .field("is_expanded", &self.is_expanded)
            .field("child", &self.child)
            .field("child_height", &self.child_height)
            .finish()
    }
}

impl WrapperWidget for TreeNodeWidget {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }
}

impl Debug for TreeWidget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeWidget")
            .finish()
    }
}

impl WrapperWidget for TreeWidget {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.root
    }
}