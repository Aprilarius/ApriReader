# ApriReader 0.9.0-beta.7 scaling smoke

Date: 2026-07-29

## Scope

An automation-assisted render smoke exercised the production frontend at these
logical viewport sizes:

- 768 x 432, representing a short high-scaling desktop layout.
- 640 x 360, exercising the compact bottom navigation.
- 320 x 360, exercising the narrowest supported shell and reader layout.

The smoke covered the library shell, Settings, reflow reader, comic/fixed
reader, compact navigation names, language access, and reader page position.
Temporary synthetic reader markup used only safe static text and was removed
after the run.

## Observed fixes

- A short icon rail now scrolls independently instead of clipping later
  destinations.
- Compact navigation keeps explicit accessible names when visual labels are
  hidden.
- The language switch remains visible outside the horizontally scrolling
  compact route list.
- Settings cards, integration statistics, language-package actions, and goal
  forms reflow without horizontal page overflow.
- Reflow reader actions move to a scrollable second toolbar row at the
  narrowest width.
- Fixed reader controls and the page selector remain visible at 320 CSS pixels.

## Result and boundary

The focused logical-viewport smoke passed. Unit coverage also verifies that
every compact route owns an accessible name independent of its visual label.

This is engineering and browser-render evidence, not the external Windows
display-scaling matrix. Before Stage 9 completion, the installed candidate
still requires product-owner or participant checks at 100%, 150%, 200%, and
250% Windows scaling on the required Windows 10/11 systems, including keyboard,
Narrator, and forced-colors passes.
