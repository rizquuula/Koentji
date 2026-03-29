use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub toast_type: ToastType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Info,
    Warning,
}

impl ToastType {
    pub fn classes(&self) -> &str {
        match self {
            ToastType::Success => "bg-green-50 border-green-400 text-green-800",
            ToastType::Error => "bg-red-50 border-red-400 text-red-800",
            ToastType::Info => "bg-blue-50 border-blue-400 text-blue-800",
            ToastType::Warning => "bg-yellow-50 border-yellow-400 text-yellow-800",
        }
    }

    pub fn icon_class(&self) -> &str {
        match self {
            ToastType::Success => "text-green-400",
            ToastType::Error => "text-red-400",
            ToastType::Info => "text-blue-400",
            ToastType::Warning => "text-yellow-400",
        }
    }
}

#[derive(Clone, Copy)]
pub struct ToastContext {
    pub toasts: ReadSignal<Vec<Toast>>,
    set_toasts: WriteSignal<Vec<Toast>>,
    next_id: RwSignal<u32>,
}

impl ToastContext {
    pub fn push(&self, message: String, toast_type: ToastType) {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let toast = Toast {
            id,
            message,
            toast_type,
        };
        self.set_toasts.update(|t| t.push(toast));

        // Auto-remove after 5 seconds
        let set_toasts = self.set_toasts;
        set_timeout(
            move || {
                set_toasts.update(|t| t.retain(|toast| toast.id != id));
            },
            std::time::Duration::from_secs(5),
        );
    }
}

pub fn provide_toast_context() -> ToastContext {
    let (toasts, set_toasts) = signal(Vec::<Toast>::new());
    let next_id = RwSignal::new(0u32);
    let ctx = ToastContext {
        toasts,
        set_toasts,
        next_id,
    };
    provide_context(ctx);
    ctx
}

pub fn use_toast() -> ToastContext {
    expect_context::<ToastContext>()
}

#[component]
pub fn ToastContainer() -> impl IntoView {
    let ctx = provide_toast_context();

    view! {
        <div class="fixed top-4 right-4 z-50 space-y-2">
            <For
                each=move || ctx.toasts.get()
                key=|toast| toast.id
                let:toast
            >
                <div class=format!(
                    "flex items-center p-4 border-l-4 rounded-lg shadow-lg max-w-sm animate-slide-in {}",
                    toast.toast_type.classes()
                )>
                    <svg class=format!("w-5 h-5 mr-3 {}", toast.toast_type.icon_class()) fill="currentColor" viewBox="0 0 20 20">
                        {match toast.toast_type {
                            ToastType::Success => view! {
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"/>
                            }.into_any(),
                            ToastType::Error => view! {
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
                            }.into_any(),
                            _ => view! {
                                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"/>
                            }.into_any(),
                        }}
                    </svg>
                    <span class="text-sm font-medium">{toast.message}</span>
                </div>
            </For>
        </div>
    }
}
