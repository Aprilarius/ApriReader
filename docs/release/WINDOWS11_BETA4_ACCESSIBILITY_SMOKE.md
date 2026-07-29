# Windows 11 beta.4 accessibility smoke

Test date: 2026-07-29  
Installed version: ApriReader `0.9.0-beta.4`  
Windows build: 10.0.22631  
Candidate SHA-256:
`2DF3AA1A9B5EF50F6BBCF75749AB1D614C78DB46F07EE5BA86B78DACAD978ABA`

This was an automation-assisted smoke test of the installed product-owner
library. It did not import, remove, rename, or modify a source book.

## Passed observations

- The library navigation exposed labelled button roles and a visible keyboard
  focus indicator.
- `Читаю сейчас` opened from the keyboard and exposed its heading, summary,
  progress, favorite control, and Continue Reading action.
- A real EPUB opened in the reflow reader without launching a console window.
- The reader toolbar exposed labelled back, chapter, contents, search,
  bookmark, annotations, and text-settings controls.
- The text-settings panel exposed labelled layout choices, font selection,
  import, typography sliders, alignment, bionic highlighting, page-wheel
  behavior, and themes.
- Book-spread mode displayed a live page range. One wheel gesture advanced from
  pages 37-38 to 39-40 and crossed into the next section.
- A right click on book text did not open a browser-authored context menu.
- The application closed normally after the test.

## Defect found and resolved in source

On beta.4, the first Tab after entering the reader could focus a chapter-footer
button and scroll the visible spread away from the restored position. Beta.5
places initial focus on the labelled top-toolbar back control in both reflow
and fixed-layout readers. Automated regression tests cover both paths.

## Not covered by this smoke

- Windows Narrator speech output.
- 150%, 200%, and 250% Windows display scaling.
- Windows forced-colors/high-contrast mode.
- Windows 10.
- Protected Steamworks behavior.
- Destructive recovery scenarios against disposable app-local data.

These cells remain required before Stage 9 can be marked complete.
