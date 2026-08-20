import { Component, type ErrorInfo, type ReactNode } from "react";

type RootErrorBoundaryProps = {
  children: ReactNode;
};

type RootErrorBoundaryState = {
  failed: boolean;
  detail: string;
};

function describeError(error: unknown) {
  if (error instanceof Error) {
    return error.message || error.name;
  }
  return typeof error === "string" ? error : String(error);
}

/**
 * Keeps a frontend exception from degrading the desktop shell into a blank
 * window. This must stay independent from application state and translations:
 * it is also the last-resort UI when those modules are the source of a fault.
 */
export class RootErrorBoundary extends Component<
  RootErrorBoundaryProps,
  RootErrorBoundaryState
> {
  state: RootErrorBoundaryState = { failed: false, detail: "" };

  static getDerivedStateFromError(error: unknown): RootErrorBoundaryState {
    return { failed: true, detail: describeError(error) };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ApriReader interface startup failed", error, errorInfo);
  }

  render() {
    if (this.state.failed) {
      return (
        <main className="root-error-screen" aria-labelledby="root-error-title">
          <section className="root-error-card">
            <span className="root-error-mark" aria-hidden="true">
              AR
            </span>
            <p className="root-error-eyebrow">APRI READER</p>
            <h1 id="root-error-title">Не удалось запустить интерфейс</h1>
            {/*
              The message has to name the actual fault. Blaming the WebView2
              runtime unconditionally sends every report down a dead end when
              the cause is an application bug, which is by far the likelier
              one.
            */}
            {this.state.detail ? (
              <p className="root-error-detail">
                <code>{this.state.detail}</code>
              </p>
            ) : null}
            <p>
              Перезапустите ApriReader. Если ошибка повторяется, сообщите текст
              выше разработчику — он определяет причину. Проверьте также, что
              установлена актуальная среда Microsoft Edge WebView2.
            </p>
            <p lang="en">
              Restart ApriReader. If this keeps happening, report the message
              above — it identifies the fault. Also check that the Microsoft
              Edge WebView2 runtime is up to date.
            </p>
            <button type="button" onClick={() => window.location.reload()}>
              Перезапустить / Restart
            </button>
          </section>
        </main>
      );
    }

    return this.props.children;
  }
}
