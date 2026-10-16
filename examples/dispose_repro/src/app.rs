use leptos::prelude::*;
use leptos_meta::{MetaTags, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{A, Route, Router, Routes},
};

/// The HTML document around [`App`].
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// The router, one route per case.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Router>
            <nav>
                <A href="/async-derived-unread">"async-derived-unread"</A>
                " | "
                <A href="/async-derived-arc">"async-derived-arc"</A>
                " | "
                <A href="/spawn">"spawn"</A>
                " | "
                <A href="/memo">"memo"</A>
            </nav>
            <main>
                <Routes fallback=|| "not found">
                    <Route path=StaticSegment("") view=|| "pick a case" />
                    <Route path=StaticSegment("async-derived") view=AsyncDerivedCase />
                    <Route path=StaticSegment("async-derived-unread") view=AsyncDerivedUnreadCase />
                    <Route path=StaticSegment("async-derived-arc") view=AsyncDerivedArcCase />
                    <Route path=StaticSegment("spawn") view=SpawnCase />
                    <Route path=StaticSegment("memo") view=MemoCase />
                </Routes>
            </main>
        </Router>
    }
}

/// Sleeps `delay_ms`, then returns three items.
#[server]
pub async fn load_items(delay_ms: u64) -> Result<Vec<u32>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    Ok(vec![1, 2, 3])
}

// Case 1a (control): an AsyncDerived awaits a Resource, then reads a signal.
// A Transition awaits the derived, so the response waits for it. Survived
// 5000 loads at 8 in parallel.
#[component]
fn AsyncDerivedCase() -> impl IntoView {
    let items = Resource::new(
        || (),
        |()| async { load_items(1).await.unwrap_or_default() },
    );
    let filter = RwSignal::new(1u32);
    let visible = AsyncDerived::new(move || async move {
        let items = items.await;
        let min = filter.get();
        items
            .into_iter()
            .filter(|item| *item >= min)
            .collect::<Vec<_>>()
    });
    view! {
        <Transition fallback=|| "loading">
            {move || Suspend::new(async move { format!("{:?}", visible.await) })}
        </Transition>
    }
}

// Case 1b (crashes): the same AsyncDerived, but nothing awaits it during SSR, as when
// it feeds a dialog that is closed on first render. The resource is awaited
// by the page's Transition, so the response finishes and the page's owner is
// disposed as soon as the resource resolves. The derived's own task wakes on
// the same resolution and then reads `filter`, which may be gone.
#[component]
fn AsyncDerivedUnreadCase() -> impl IntoView {
    let items = Resource::new(
        || (),
        |()| async { load_items(1).await.unwrap_or_default() },
    );
    let filter = RwSignal::new(1u32);
    let open = RwSignal::new(false);
    let visible = AsyncDerived::new(move || async move {
        let items = items.await;
        let min = filter.get();
        items
            .into_iter()
            .filter(|item| *item >= min)
            .collect::<Vec<_>>()
    });
    view! {
        <Transition fallback=|| "loading">
            {move || Suspend::new(async move { format!("{} items", items.await.len()) })}
        </Transition>
        <Show when=move || open.get()>
            <Transition>
                {move || Suspend::new(async move { format!("{:?}", visible.await) })}
            </Transition>
        </Show>
    }
}

// Case 1c (survives): case 1b with the workaround. The closure captures refcounted
// handles converted at setup, which a disposed owner cannot take away.
#[component]
fn AsyncDerivedArcCase() -> impl IntoView {
    let items = Resource::new(
        || (),
        |()| async { load_items(1).await.unwrap_or_default() },
    );
    let filter = RwSignal::new(1u32);
    let open = RwSignal::new(false);
    let visible = {
        let items = ArcResource::from(items);
        let filter = ArcRwSignal::from(filter);
        AsyncDerived::new(move || {
            let (items, filter) = (items.clone(), filter.clone());
            async move {
                let items = items.await;
                let min = filter.get();
                items
                    .into_iter()
                    .filter(|item| *item >= min)
                    .collect::<Vec<_>>()
            }
        })
    };
    view! {
        <Transition fallback=|| "loading">
            {move || Suspend::new(async move { format!("{} items", items.await.len()) })}
        </Transition>
        <Show when=move || open.get()>
            <Transition>
                {move || Suspend::new(async move { format!("{:?}", visible.await) })}
            </Transition>
        </Show>
    }
}

// Case 2: a click handler spawns a request and reads a signal after the
// await. Closing the dialog while the request is in flight disposes the
// signal, and the read after the await panics.
#[component]
fn SpawnCase() -> impl IntoView {
    let open = RwSignal::new(true);
    view! {
        <button id="toggle" on:click=move |_| open.update(|open| *open = !*open)>"toggle dialog"</button>
        <Show when=move || open.get()>
            <Dialog />
        </Show>
    }
}

#[component]
fn Dialog() -> impl IntoView {
    let status = RwSignal::new("idle".to_string());
    let on_click = move |_| {
        status.set("loading".to_string());
        leptos::task::spawn_local(async move {
            let items = load_items(3000).await.unwrap_or_default();
            // `set` on a disposed signal is a silent no-op, but this read panics.
            let previous = status.get_untracked();
            status.set(format!("{previous} -> {} items", items.len()));
        });
    };
    view! {
        <div style="border: 1px solid; padding: 1em">
            <button id="load" on:click=on_click>"load (3s)"</button>
            <span>{move || status.get()}</span>
        </div>
    }
}

// Case 3: a Memo over a resource, built outside the Transition that renders
// it. The Memo's resource read is not registered with the Transition, so the
// server renders the resource as it is when the shell flushes, while
// the resource resolves later. Here hydration keeps the server's branch with no
// error. In a larger page the same shape panicked in tachys hydration.rs:163.
#[component]
fn MemoCase() -> impl IntoView {
    let items = Resource::new(
        || (),
        |()| async { load_items(1).await.unwrap_or_default() },
    );
    let has_items =
        Memo::new(move |_| items.get().is_some_and(|items| !items.is_empty()));
    view! {
        <Transition fallback=|| "loading">
            <Show when=move || has_items.get() fallback=|| view! { <span id="empty">"empty"</span> }>
                <div id="items">"has items"</div>
            </Show>
        </Transition>
    }
}
