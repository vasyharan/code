# Introduction

**Toku** was created by a Vim user that grew increasing envious of VS Code users (remote code development, debugger, git integration, etc). 

The primary goals of Toku are:
1. Responsiveness.
   - The time from user input to visual feedback must be under 16ms (time to render at 60fps).
2. Terminal & keyboard first.
   - Supports running in a VT100 terminal.
   - Every feature of Toku **must** be accessible without a mouse.
   - There are lots other great code editors ([VS Code](https://code.visualstudio.com/), [Zed](https://zed.dev/), [Lapce](https://lapce.dev/)), but they are not terminal friendly, and most require using a mouse.
3. Modal editing.
   - Built with first class support for modal editing.
4. Discoverable.
   - Feature are easily discoverable, and usable.
   - _(Vim is very powerful, but I've most myself constantly searching for the magic incantations for using a infrequently used feature, like deleting matching lines.)_
5. Enable developer productivity.
   - Built in productivity tooling _(no more installing/configuring surround, fugitive, fzf, nvim-cmp, etc.)_