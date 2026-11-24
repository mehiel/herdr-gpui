//! One agent looks selected in a window however many hosts it lists: the
//! pane the window is in. Each daemon reports the pane last focused on it,
//! so a second host (a machine saved for the window's own session, say)
//! brings a second focused agent that must not be marked.
use super::*;
use crate::endpoint::Endpoint;
use sheprd_client::ConnectTarget;

const HOST: &str = "ssh:saved";

/// A window on this computer, whose agent `p0` is focused, beside a saved
/// host whose own daemon reports its agent `r0` focused.
fn window_beside_host(
    cx: &mut TestAppContext,
) -> (gpui::Entity<SheprdWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, _| {
        let mut local = crate::sidebar::layout_tests::snapshot(2);
        local.agents[0].focused = true;
        let local = Some(Arc::new(local));
        // What the window draws while it is here, and what this host's own
        // connection keeps once the window goes elsewhere.
        view.endpoints[0].live.snapshot = local.clone();
        view.live.snapshot = local;
        let mut remote = crate::sidebar::layout_tests::snapshot(2);
        remote.agents[0].pane_id = "r0".into();
        remote.agents[0].focused = true;
        remote.agents[1].pane_id = "r1".into();
        let mut endpoint = Endpoint::new(
            HOST.into(),
            "Saved".into(),
            ConnectTarget::Ssh {
                target: "me@saved".into(),
                session: "default".into(),
            },
            true,
        );
        endpoint.never_connect();
        endpoint.live.snapshot = Some(Arc::new(remote));
        view.endpoints.push(endpoint);
    });
    (view, cx)
}

/// The panes the window marks as the one it is in: Moonframe's agents list
/// and the classic one read the same rule.
fn marked(view: &mut SheprdWindow, cx: &mut gpui::Context<SheprdWindow>) -> Vec<String> {
    let state = view.hosted_state(cx);
    let hosted: Vec<String> = state.main["agents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|agent| agent["focused"] == true)
        .map(|agent| agent["pane"].as_str().unwrap().to_owned())
        .collect();
    let classic: Vec<String> = view
        .panel_agents()
        .into_iter()
        .filter(|(index, agent)| view.agent_in_use(*index, agent))
        .map(|(_, agent)| agent.id().to_owned())
        .collect();
    assert_eq!(hosted, classic, "both windows mark the same agent");
    hosted
}

#[gpui::test]
fn only_the_pane_the_window_is_in_is_marked(cx: &mut TestAppContext) {
    let (view, cx) = window_beside_host(cx);
    view.update(cx, |view, cx| {
        let state = view.hosted_state(cx);
        let agents = state.main["agents"].as_array().unwrap();
        assert!(
            agents.iter().any(|agent| agent["pane"] == "r0"),
            "the saved host's agents are listed: {agents:?}"
        );
        assert_eq!(marked(view, cx), ["p0"]);
    });
}

#[gpui::test]
fn clicking_another_hosts_agent_moves_the_one_mark_there(cx: &mut TestAppContext) {
    let (view, cx) = window_beside_host(cx);
    view.update_in(cx, |view, window, cx| {
        view.hosted_action(
            Action::FocusPane {
                endpoint: Some(HOST.into()),
                pane: "r0".into(),
            },
            window,
            cx,
        );
        assert_eq!(
            view.selected_endpoint, 1,
            "the window went to the saved host"
        );
        let state = view.hosted_state(cx);
        assert!(
            state.main["agents"]
                .as_array()
                .unwrap()
                .iter()
                .any(|agent| agent["pane"] == "p0"),
            "this computer's agents stay listed"
        );
        assert_eq!(marked(view, cx), ["r0"]);
    });
}
