package io.github.aminuddinihsan20.bitchord;

import android.Manifest;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.KeyEvent;
import android.view.View;

import androidx.activity.OnBackPressedCallback;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;

import com.google.androidgamesdk.GameActivity;

/**
 * Loads libbitchord_android.so (named in the manifest), whose android_main runs the app,
 * and tells it what only Java can: the system bars' size and the back button.
 */
public class MainActivity extends GameActivity {
    // Implemented in android/rust/src/lib.rs.
    private static native void nativeInsets(int left, int top, int right, int bottom);

    private static native boolean nativeCanGoBack();

    private static native void nativeBack();

    private static native void nativeEnter();

    /** A sign-in the app asked for: 1 Google, 2 Spotify, 0 none. */
    private static native int nativeTakeLoginRequest();

    /** The app is away (stopped) or back: the music is kept going from a thread while away. */
    private static native void nativeSetBackground(boolean away);

    /** What plays, for the media notification; empty when nothing is loaded. */
    private static native String nativeNowPlaying();

    private boolean hadMedia;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Edge to edge: the app draws under the status and gesture bars and keeps its
        // content clear of them (and above the keyboard) by these insets.
        View decor = getWindow().getDecorView();
        ViewCompat.setOnApplyWindowInsetsListener(decor, (view, insets) -> {
            Insets bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() | WindowInsetsCompat.Type.displayCutout());
            Insets ime = insets.getInsets(WindowInsetsCompat.Type.ime());
            nativeInsets(bars.left, bars.top, bars.right, Math.max(bars.bottom, ime.bottom));
            return insets;
        });
        ViewCompat.requestApplyInsets(decor);

        // The app asks for a sign-in by setting a request; open the sign-in page for it.
        Handler main = new Handler(Looper.getMainLooper());
        main.post(new Runnable() {
            @Override
            public void run() {
                String playing = nativeNowPlaying();
                boolean media = playing != null && !playing.isEmpty();
                if (media && !hadMedia) {
                    startPlaybackService();
                }
                hadMedia = media;
                int site = nativeTakeLoginRequest();
                if (site != 0) {
                    startActivity(new Intent(MainActivity.this, LoginActivity.class).putExtra(LoginActivity.EXTRA_SITE, site));
                }
                main.postDelayed(this, 250);
            }
        });

        // Back closes what the app has open (Now Playing, then a page); with nothing to
        // close the app goes to the background and keeps playing, as music apps do.
        getOnBackPressedDispatcher().addCallback(this, new OnBackPressedCallback(true) {
            @Override
            public void handleOnBackPressed() {
                if (nativeCanGoBack()) {
                    nativeBack();
                } else {
                    moveTaskToBack(true);
                }
            }
        });
    }

    @Override
    protected void onStart() {
        super.onStart();
        nativeSetBackground(false);
    }

    @Override
    protected void onStop() {
        nativeSetBackground(true);
        super.onStop();
    }

    /** The first song: ask once for notifications (Android 13+), then show the player. */
    private void startPlaybackService() {
        if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, 1);
        }
        startForegroundService(new Intent(this, PlaybackService.class));
    }

    // winit reports Enter as the character '\n', which the app's text fields ignore: hand
    // it over as the Enter key (the keyboard's action key sends it too).
    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.getKeyCode() == KeyEvent.KEYCODE_ENTER) {
            if (event.getAction() == KeyEvent.ACTION_DOWN && event.getRepeatCount() == 0) {
                nativeEnter();
            }
            return true;
        }
        return super.dispatchKeyEvent(event);
    }
}
