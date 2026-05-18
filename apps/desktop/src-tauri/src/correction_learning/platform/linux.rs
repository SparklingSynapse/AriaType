use atspi::connection::{AccessibilityConnection, P2P};
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::text::TextProxy;
use atspi::zbus::names::BusName;
use atspi::zbus::proxy::CacheProperties;
use atspi::{Interface, ObjectRefOwned, State};
use std::collections::{HashSet, VecDeque};

const MAX_ACCESSIBILITY_NODES: usize = 512;

pub async fn read_focused_editable_text() -> Option<String> {
    let connection = AccessibilityConnection::new().await.ok()?;
    let root = connection.root_accessible_on_registry().await.ok()?;
    let focused = find_focused_text_object(&connection, root).await?;

    read_text_from_object(&connection, &focused).await
}

async fn find_focused_text_object(
    connection: &AccessibilityConnection,
    root: AccessibleProxy<'_>,
) -> Option<ObjectRefOwned> {
    let mut queue = VecDeque::new();
    let root_ref = ObjectRefOwned::try_from(&root).ok()?;
    queue.push_back(root_ref);

    let mut visited = HashSet::new();
    let mut visited_count = 0usize;

    while let Some(object_ref) = queue.pop_front() {
        if visited_count >= MAX_ACCESSIBILITY_NODES || !visited.insert(object_ref.clone()) {
            continue;
        }
        visited_count += 1;

        let accessible = connection.object_as_accessible(&object_ref).await.ok()?;
        if is_focused_text_candidate(&accessible).await {
            return Some(object_ref);
        }

        if let Ok(children) = accessible.get_children().await {
            for child in children {
                if !child.is_null() {
                    queue.push_back(child);
                }
            }
        }
    }

    None
}

async fn is_focused_text_candidate(accessible: &AccessibleProxy<'_>) -> bool {
    let Ok(states) = accessible.get_state().await else {
        return false;
    };
    if !states.contains(State::Focused) {
        return false;
    }

    let Ok(interfaces) = accessible.get_interfaces().await else {
        return false;
    };

    interfaces.contains(Interface::Text) || interfaces.contains(Interface::EditableText)
}

async fn read_text_from_object(
    connection: &AccessibilityConnection,
    object_ref: &ObjectRefOwned,
) -> Option<String> {
    let text_proxy = text_proxy_for_object(connection, object_ref).await?;
    let character_count = text_proxy.character_count().await.ok()?;
    let caret_offset = text_proxy.caret_offset().await.ok();
    let (start, end) = super::bounded_text_range(character_count, caret_offset)?;

    text_proxy
        .get_text(start, end)
        .await
        .ok()
        .and_then(super::non_empty_text)
}

async fn text_proxy_for_object(
    connection: &AccessibilityConnection,
    object_ref: &ObjectRefOwned,
) -> Option<TextProxy<'_>> {
    if object_ref.is_null() {
        return None;
    }

    let name: BusName = object_ref.name()?.clone().into();
    TextProxy::builder(connection.connection())
        .destination(name)
        .ok()?
        .path(object_ref.path())
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()
}
