package com.saveeditoroffline.app;

import android.app.NativeActivity;
import android.content.Context;
import android.os.Bundle;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

/**
 * NativeActivity has no text-input connection, so on-screen keyboards can't type into it.
 * This adds an invisible view that receives the keyboard's text and hands it to the Rust app.
 */
public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("save_editor_offline");
    }

    /** Typed text. */
    static native void nativeText(String text);

    /** A key without text (Android key code): backspace, enter, arrows. */
    static native void nativeKey(int keyCode);

    private KeyboardView keyboardView;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        keyboardView = new KeyboardView(this);
        addContentView(keyboardView, new ViewGroup.LayoutParams(1, 1));
    }

    /** Called from Rust when a text field gains or loses focus. */
    public void setKeyboard(final boolean show) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
                if (show) {
                    keyboardView.requestFocus();
                    imm.restartInput(keyboardView);
                    imm.showSoftInput(keyboardView, 0);
                } else {
                    imm.hideSoftInputFromWindow(keyboardView.getWindowToken(), 0);
                }
            }
        });
    }

    static class KeyboardView extends View {
        KeyboardView(Context c) {
            super(c);
            setFocusable(true);
            setFocusableInTouchMode(true);
        }

        @Override
        public boolean onCheckIsTextEditor() {
            return true;
        }

        @Override
        public InputConnection onCreateInputConnection(EditorInfo out) {
            // Plain characters, no suggestions or auto-correct: game names and numbers.
            out.inputType = InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                    | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS;
            out.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_FLAG_NO_FULLSCREEN | EditorInfo.IME_ACTION_DONE;
            return new Connection(this);
        }
    }

    /** Forwards everything the keyboard does; the app's own text field holds the text. */
    static class Connection extends BaseInputConnection {
        /** Length of the word being composed, already sent to the app. */
        private int composing = 0;

        Connection(View v) {
            super(v, false);
        }

        private void replaceComposing(CharSequence text) {
            for (int i = 0; i < composing; i++) {
                nativeKey(KeyEvent.KEYCODE_DEL);
            }
            composing = 0;
            if (text != null && text.length() > 0) {
                nativeText(text.toString());
            }
        }

        @Override
        public boolean commitText(CharSequence text, int newCursorPosition) {
            replaceComposing(text);
            return true;
        }

        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            replaceComposing(text);
            composing = text == null ? 0 : text.length();
            return true;
        }

        @Override
        public boolean finishComposingText() {
            composing = 0;
            return true;
        }

        @Override
        public boolean deleteSurroundingText(int before, int after) {
            for (int i = 0; i < before; i++) {
                nativeKey(KeyEvent.KEYCODE_DEL);
            }
            for (int i = 0; i < after; i++) {
                nativeKey(KeyEvent.KEYCODE_FORWARD_DEL);
            }
            return true;
        }

        @Override
        public boolean sendKeyEvent(KeyEvent e) {
            if (e.getAction() == KeyEvent.ACTION_DOWN) {
                int ch = e.getUnicodeChar();
                if (ch != 0 && !Character.isISOControl(ch)) {
                    nativeText(String.valueOf((char) ch));
                } else {
                    nativeKey(e.getKeyCode());
                }
            }
            return true;
        }

        @Override
        public boolean performEditorAction(int action) {
            nativeKey(KeyEvent.KEYCODE_ENTER);
            return true;
        }
    }
}
