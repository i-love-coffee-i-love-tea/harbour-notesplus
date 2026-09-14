use std::collections::HashMap;
use std::path::Path;

use crate::group::GroupInfo;
use crate::html::qt_html::{QtRenderOptions, QtThemeColors};
use crate::page;

/// Builds a hierarchical JSON tree of groups and their pages for the main page UI.
///
/// Returns a JSON string representing an array of top-level group nodes, each with
/// nested `pages` and `children` arrays.
pub fn build_group_tree(
    pages: &[page::PageInfo],
    groups: &[GroupInfo],
    max_depth: i32,
    notes_dir: Option<&Path>,
    drop_comments: bool,
    qt_theme: Option<&QtThemeColors>,
    qt_options: Option<&QtRenderOptions>,
) -> String {
    let non_journal_pages: Vec<&page::PageInfo> = pages.iter().filter(|p| !p.is_journal).collect();

    let mut pages_by_group: HashMap<String, Vec<&page::PageInfo>> = HashMap::new();
    for p in &non_journal_pages {
        pages_by_group
            .entry(p.group_path.clone())
            .or_default()
            .push(p);
    }

    let mut groups_by_path: HashMap<String, &GroupInfo> = HashMap::new();
    for g in groups {
        groups_by_path.insert(g.path.clone(), g);
    }

    eprintln!(
        "[debug] build_group_tree: pages_by_group keys: {:?}",
        pages_by_group.keys().collect::<Vec<_>>()
    );
    eprintln!(
        "[debug] build_group_tree: groups_by_path keys: {:?}",
        groups_by_path.keys().collect::<Vec<_>>()
    );

    let mut top_level_groups: Vec<String> = groups
        .iter()
        .filter(|g| !g.path.is_empty() && !g.path.contains('/'))
        .map(|g| g.path.clone())
        .collect();

    for p in &non_journal_pages {
        if !p.group_path.is_empty() {
            let top = p.group_path.split('/').next().unwrap().to_string();
            if !top_level_groups.contains(&top) {
                top_level_groups.push(top);
            }
        }
    }

    // Guarantee all DB groups appear in the tree (even empty ones not referenced by any page)
    for g in groups {
        if g.path.is_empty() {
            continue;
        }
        let top = if let Some(slash_pos) = g.path.find('/') {
            &g.path[..slash_pos]
        } else {
            &g.path
        };
        if !top_level_groups.contains(&top.to_string()) {
            top_level_groups.push(top.to_string());
        }
    }

    top_level_groups.sort();
    eprintln!(
        "[debug] build_group_tree: top_level_groups={:?}",
        top_level_groups
    );

    let mut root_trees = Vec::new();

    // Include ungrouped root pages under "Notes" if any exist
    if let Some(root_pages) = pages_by_group.get("") {
        if !root_pages.is_empty() {
            let collapsed = groups_by_path
                .get("")
                .map(|g| g.collapsed)
                .unwrap_or(false);
            root_trees.push(build_node(
                "",
                "Notes",
                collapsed,
                0,
                max_depth,
                &groups_by_path,
                &pages_by_group,
                notes_dir,
                drop_comments,
                qt_theme,
                qt_options,
            ));
        }
    }

    let debug_groups = top_level_groups.clone();

    for top_path in top_level_groups {
        let disp = groups_by_path
            .get(&top_path)
            .map(|g| g.display_name.clone())
            .unwrap_or_else(|| top_path.clone());

        let collapsed = groups_by_path
            .get(&top_path)
            .map(|g| g.collapsed)
            .unwrap_or(false);

        root_trees.push(build_node(
            &top_path,
            &disp,
            collapsed,
            1,
            max_depth,
            &groups_by_path,
            &pages_by_group,
            notes_dir,
            drop_comments,
            qt_theme,
            qt_options,
        ));
    }

    let result =
        serde_json::to_string(&root_trees).unwrap_or_else(|_| "[]".to_string());
    eprintln!(
        "[debug] build_group_tree: {} top_level_groups={:?}, {} root_trees, result len={}",
        debug_groups.len(),
        debug_groups,
        root_trees.len(),
        result.len()
    );
    result
}

#[allow(clippy::too_many_arguments)]
fn build_node(
    group_path: &str,
    display_name: &str,
    collapsed: bool,
    current_depth: i32,
    max_depth: i32,
    groups_by_path: &HashMap<String, &GroupInfo>,
    pages_by_group: &HashMap<String, Vec<&page::PageInfo>>,
    notes_dir: Option<&Path>,
    drop_comments: bool,
    qt_theme: Option<&QtThemeColors>,
    qt_options: Option<&QtRenderOptions>,
) -> serde_json::Value {
    let empty_pages = Vec::new();
    let my_pages = pages_by_group
        .get(group_path)
        .unwrap_or(&empty_pages);
    let note_sort = groups_by_path
        .get(group_path)
        .map(|g| g.note_sort)
        .unwrap_or_default();

    let mut my_pages_sorted = my_pages.clone();
    match note_sort {
        crate::group::NoteSortOrder::ByName => {
            my_pages_sorted.sort_by(|a, b| {
                let name_a = a.title.to_lowercase();
                let name_b = b.title.to_lowercase();
                name_a.cmp(&name_b).then_with(|| a.title.cmp(&b.title))
            });
        }
        crate::group::NoteSortOrder::NewestFirst => {
            my_pages_sorted.sort_by(|a, b| {
                b.updated_at.cmp(&a.updated_at).then_with(|| b.id.cmp(&a.id))
            });
        }
    }

    eprintln!(
        "[debug] build_node: group_path='{}' depth={} pages_found={} note_sort={:?} groups_by_path_keys={:?}",
        group_path,
        current_depth,
        my_pages.len(),
        note_sort,
        groups_by_path.keys().collect::<Vec<_>>()
    );

    let mut page_json_list = Vec::new();
    for p in my_pages_sorted.iter() {
        let mut p_map = serde_json::Map::new();
        p_map.insert("id".into(), serde_json::Value::Number(p.id.into()));
        p_map.insert(
            "name".into(),
            serde_json::Value::String(p.title.clone()),
        );
        p_map.insert(
            "filename".into(),
            serde_json::Value::String(p.filename.clone()),
        );
        p_map.insert(
            "group_path".into(),
            serde_json::Value::String(p.group_path.clone()),
        );
        p_map.insert(
            "full_path".into(),
            serde_json::Value::String(p.full_path()),
        );
        p_map.insert(
            "created_at".into(),
            serde_json::Value::String(p.created_at.clone()),
        );
        p_map.insert(
            "updated_at".into(),
            serde_json::Value::String(p.updated_at.clone()),
        );
        p_map.insert(
            "block_count".into(),
            serde_json::Value::Number(p.block_count.into()),
        );

        let preview_values = if let Some(dir) = notes_dir {
            if !collapsed {
                page::get_page_preview_values_with_options(
                    dir,
                    &p.full_path(),
                    8,
                    drop_comments,
                    qt_theme,
                    qt_options,
                )
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let preview_json_str =
            serde_json::to_string(&preview_values).unwrap_or_else(|_| "[]".to_string());
        p_map.insert(
            "preview_blocks".into(),
            serde_json::Value::Array(preview_values),
        );
        p_map.insert(
            "preview_blocks_json".into(),
            serde_json::Value::String(preview_json_str),
        );

        page_json_list.push(serde_json::Value::Object(p_map));
    }

    let prefix = if group_path.is_empty() {
        "".to_string()
    } else {
        format!("{}/", group_path)
    };
    let mut child_nodes = Vec::new();

    if !group_path.is_empty() && (current_depth < max_depth || max_depth <= 0) {
        let mut direct_child_paths: Vec<String> = groups_by_path
            .keys()
            .filter(|p| {
                if let Some(rest) = p.strip_prefix(&prefix) {
                    !rest.contains('/')
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        for p_path in pages_by_group.keys() {
            if let Some(rest) = p_path.strip_prefix(&prefix) {
                if let Some(child_seg) = rest.split('/').next() {
                    let full_child = format!("{}{}", prefix, child_seg);
                    if !direct_child_paths.contains(&full_child) {
                        direct_child_paths.push(full_child);
                    }
                }
            }
        }

        direct_child_paths.sort();
        eprintln!(
            "[debug] build_node: group_path='{}' direct_child_paths={:?}",
            group_path, direct_child_paths
        );

        for c_path in direct_child_paths {
            let c_disp = groups_by_path
                .get(&c_path)
                .map(|g| g.display_name.clone())
                .unwrap_or_else(|| {
                    c_path
                        .rsplit('/')
                        .next()
                        .unwrap_or(&c_path)
                        .to_string()
                });

            let c_collapsed = groups_by_path
                .get(&c_path)
                .map(|g| g.collapsed)
                .unwrap_or(false);

            child_nodes.push(build_node(
                &c_path,
                &c_disp,
                c_collapsed,
                current_depth + 1,
                max_depth,
                groups_by_path,
                pages_by_group,
                notes_dir,
                drop_comments,
                qt_theme,
                qt_options,
            ));
        }
    }

    let mut node = serde_json::Map::new();
    node.insert(
        "path".into(),
        serde_json::Value::String(group_path.to_string()),
    );
    node.insert(
        "display_name".into(),
        serde_json::Value::String(display_name.to_string()),
    );
    node.insert("collapsed".into(), serde_json::Value::Bool(collapsed));
    node.insert(
        "depth".into(),
        serde_json::Value::Number(current_depth.into()),
    );
    node.insert(
        "note_count".into(),
        serde_json::Value::Number(my_pages.len().into()),
    );
    node.insert(
        "child_group_count".into(),
        serde_json::Value::Number(child_nodes.len().into()),
    );
    node.insert(
        "note_sort".into(),
        serde_json::Value::String(note_sort.as_str().to_string()),
    );
    node.insert(
        "pages".into(),
        serde_json::Value::Array(page_json_list),
    );
    node.insert(
        "children".into(),
        serde_json::Value::Array(child_nodes),
    );

    serde_json::Value::Object(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::GroupInfo;
    use crate::page::PageInfo;

    fn make_page(id: i64, filename: &str, group_path: &str, title: &str) -> PageInfo {
        PageInfo {
            id,
            filename: filename.into(),
            group_path: group_path.into(),
            title: title.into(),
            is_journal: false,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            block_count: 0,
        }
    }

    fn make_group(path: &str, display_name: &str) -> GroupInfo {
        GroupInfo {
            path: path.into(),
            display_name: display_name.into(),
            note_count: 0,
            child_group_count: 0,
            collapsed: false,
            sort_order: 0,
            note_sort: crate::group::NoteSortOrder::NewestFirst,
        }
    }

    #[test]
    fn test_group1_with_direct_pages_and_sub() {
        let pages = vec![
            make_page(1, "Note1.adoc", "Group1", "Note1"),
            make_page(2, "Note2.adoc", "Group1", "Note2"),
            make_page(3, "SubNote.adoc", "Group1/Sub", "SubNote"),
        ];

        let groups = vec![
            make_group("Group1", "Group1"),
            make_group("Group1/Sub", "Sub"),
        ];

        let tree_json = build_group_tree(&pages, &groups, 5, None, true, None, None);
        let parsed: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1); // Only Group1 (no ungrouped pages)

        let group1 = &arr[0];
        assert_eq!(group1["path"], "Group1");
        assert_eq!(group1["display_name"], "Group1");

        let p = group1["pages"].as_array().unwrap();
        assert_eq!(p.len(), 2, "Group1 should have 2 direct pages");

        let children = group1["children"].as_array().unwrap();
        assert_eq!(children.len(), 1, "Group1 should have 1 child group");
        assert_eq!(children[0]["path"], "Group1/Sub");
        assert_eq!(children[0]["display_name"], "Sub");

        let sub_pages = children[0]["pages"].as_array().unwrap();
        assert_eq!(sub_pages.len(), 1, "Sub should have 1 page");
    }

    #[test]
    fn test_group1_with_no_direct_pages_but_sub_has_pages() {
        // Group1 exists but has no direct pages; only Sub has pages
        let pages = vec![make_page(1, "SubNote.adoc", "Group1/Sub", "SubNote")];

        let groups = vec![
            make_group("Group1", "Group1"),
            make_group("Group1/Sub", "Sub"),
        ];

        let tree_json = build_group_tree(&pages, &groups, 5, None, true, None, None);
        let parsed: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let group1 = &arr[0];
        assert_eq!(group1["path"], "Group1");

        let p = group1["pages"].as_array().unwrap();
        assert_eq!(p.len(), 0, "Group1 has no direct pages");

        let children = group1["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["path"], "Group1/Sub");

        let sub_pages = children[0]["pages"].as_array().unwrap();
        assert_eq!(sub_pages.len(), 1);
    }

    #[test]
    fn test_sub_discovered_from_pages_alone() {
        // Group1/Sub only exists in pages table (no explicit group entry)
        let pages = vec![
            make_page(1, "Note1.adoc", "Group1", "Note1"),
            make_page(2, "SubNote.adoc", "Group1/Sub", "SubNote"),
        ];

        let groups = vec![make_group("Group1", "Group1")];
        // Note: no "Group1/Sub" group entry

        let tree_json = build_group_tree(&pages, &groups, 5, None, true, None, None);
        let parsed: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let group1 = &arr[0];
        let p = group1["pages"].as_array().unwrap();
        assert_eq!(p.len(), 1, "Group1 direct page");

        let children = group1["children"].as_array().unwrap();
        assert_eq!(
            children.len(),
            1,
            "Sub should be discovered from pages alone"
        );
        assert_eq!(children[0]["path"], "Group1/Sub");

        let sub_pages = children[0]["pages"].as_array().unwrap();
        assert_eq!(sub_pages.len(), 1);
    }

    #[test]
    fn test_multiple_top_level_groups_with_nesting() {
        let pages = vec![
            make_page(1, "A.adoc", "Alpha", "A"),
            make_page(2, "B.adoc", "Alpha/Child", "B"),
            make_page(3, "C.adoc", "Beta", "C"),
            make_page(4, "D.adoc", "Beta/Sub/Deep", "D"),
        ];

        let groups = vec![
            make_group("Alpha", "Alpha"),
            make_group("Alpha/Child", "Child"),
            make_group("Beta", "Beta"),
            make_group("Beta/Sub", "Sub"),
            make_group("Beta/Sub/Deep", "Deep"),
        ];

        let tree_json = build_group_tree(&pages, &groups, 5, None, true, None, None);
        let parsed: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2); // Alpha + Beta

        let alpha = &arr[0];
        assert_eq!(alpha["path"], "Alpha");
        assert_eq!(alpha["pages"].as_array().unwrap().len(), 1);
        assert_eq!(alpha["children"].as_array().unwrap().len(), 1);
        assert_eq!(alpha["children"][0]["path"], "Alpha/Child");

        let beta = &arr[1];
        assert_eq!(beta["path"], "Beta");
        assert_eq!(beta["pages"].as_array().unwrap().len(), 1);
        let beta_children = beta["children"].as_array().unwrap();
        assert_eq!(beta_children.len(), 1);
        assert_eq!(beta_children[0]["path"], "Beta/Sub");
        // Beta/Sub should have Beta/Sub/Deep as a child
        let sub_children = beta_children[0]["children"].as_array().unwrap();
        assert_eq!(sub_children.len(), 1);
        assert_eq!(sub_children[0]["path"], "Beta/Sub/Deep");
    }

    #[test]
    fn test_ungrouped_and_grouped_pages() {
        let pages = vec![
            make_page(1, "Root.adoc", "", "Root Note"),
            make_page(2, "Work.adoc", "Work", "Work Note"),
        ];

        let groups = vec![make_group("Work", "Work")];

        let tree_json = build_group_tree(&pages, &groups, 5, None, true, None, None);
        let parsed: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
        let arr = parsed.as_array().unwrap();

        assert_eq!(arr.len(), 2); // Recent Notes + Work

        let recent = &arr[0];
        assert_eq!(recent["path"], "");
        assert_eq!(recent["pages"].as_array().unwrap().len(), 1);

        let work = &arr[1];
        assert_eq!(work["path"], "Work");
        assert_eq!(work["pages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_collapsed_groups_and_ungrouped() {
        let pages = vec![
            make_page(1, "Root.adoc", "", "Root Note"),
            make_page(2, "Work.adoc", "Work", "Work Note"),
        ];

        let mut root_group = make_group("", "Notes");
        root_group.collapsed = true;

        let mut work_group = make_group("Work", "Work");
        work_group.collapsed = true;

        let groups = vec![root_group, work_group];

        let tree_json = build_group_tree(&pages, &groups, 5, None, true, None, None);
        let parsed: serde_json::Value = serde_json::from_str(&tree_json).unwrap();
        let arr = parsed.as_array().unwrap();

        assert_eq!(arr.len(), 2);

        let recent = &arr[0];
        assert_eq!(recent["path"], "");
        assert_eq!(recent["collapsed"], true);

        let work = &arr[1];
        assert_eq!(work["path"], "Work");
        assert_eq!(work["collapsed"], true);
    }

    #[test]
    fn test_group_note_sorting_by_name_and_newest() {
        let mut p1 = make_page(1, "Banana.adoc", "Fruits", "Banana");
        p1.updated_at = "2026-01-01T10:00:00Z".into();

        let mut p2 = make_page(2, "Apple.adoc", "Fruits", "Apple");
        p2.updated_at = "2026-01-02T10:00:00Z".into();

        let mut p3 = make_page(3, "Cherry.adoc", "Fruits", "Cherry");
        p3.updated_at = "2026-01-03T10:00:00Z".into();

        let pages = vec![p1, p2, p3];

        // 1. Test newest first (default)
        let mut group_newest = make_group("Fruits", "Fruits");
        group_newest.note_sort = crate::group::NoteSortOrder::NewestFirst;

        let tree_json_newest = build_group_tree(&pages, &[group_newest], 5, None, true, None, None);
        let parsed_newest: serde_json::Value = serde_json::from_str(&tree_json_newest).unwrap();
        let pages_newest = parsed_newest[0]["pages"].as_array().unwrap();
        assert_eq!(pages_newest.len(), 3);
        assert_eq!(pages_newest[0]["name"], "Cherry"); // newest updated_at
        assert_eq!(pages_newest[1]["name"], "Apple");
        assert_eq!(pages_newest[2]["name"], "Banana");
        assert_eq!(parsed_newest[0]["note_sort"], "newest");

        // 2. Test by name
        let mut group_by_name = make_group("Fruits", "Fruits");
        group_by_name.note_sort = crate::group::NoteSortOrder::ByName;

        let tree_json_by_name = build_group_tree(&pages, &[group_by_name], 5, None, true, None, None);
        let parsed_by_name: serde_json::Value = serde_json::from_str(&tree_json_by_name).unwrap();
        let pages_by_name = parsed_by_name[0]["pages"].as_array().unwrap();
        assert_eq!(pages_by_name.len(), 3);
        assert_eq!(pages_by_name[0]["name"], "Apple"); // alphabetical
        assert_eq!(pages_by_name[1]["name"], "Banana");
        assert_eq!(pages_by_name[2]["name"], "Cherry");
        assert_eq!(parsed_by_name[0]["note_sort"], "name");
    }
}
