Ascii Math Unicode
==================

[![build](https://github.com/hafacc/asciimath-unicode/actions/workflows/ext-build.yml/badge.svg)](https://github.com/hafacc/asciimath-unicode/actions/workflows/ext-build.yml)
[![chrome](https://img.shields.io/badge/chrome-extension-orange)](https://chrome.google.com/webstore/detail/llehdcbaonklonjlfgeggamnebgggoab?authuser=0&hl=en)
[![license](https://img.shields.io/github/license/hafacc/asciimath-unicode)](LICENSE)

A chrome extension for rendering [ascii math](http://asciimath.org/) as unicode.

Clicking the extension's icon, or pressing its hotkey (Alt+Shift+U, or Control+Shift+U on Mac), turns on conversion as you type for the current page until you reload it or leave.
With "Stay on for the whole site" in the options, it stays on for the site instead, after Chrome asks for access to it.
Finishing a delimited span in a text box replaces it with unicode: `$$x^2$$` converts as soon as the closing `$$` is typed, as do complete spans in pasted text. Undo restores the original, which then stays as typed.
The icon is grey where this is off.
The delimiters (`$$…$$` by default) are in the options.

For example, typing the following (the example from ascii math):

```
$$sum_(i=1)^n i^3=((n(n+1))/2)^2$$
```

You'll convert it to:
```
∑ᵢ₌₁ⁿi³=(ⁿ⁽ⁿ⁺¹⁾⁄₂)²
```

While not a perfect representation, this is more portable than an image, and can often be easier to read.

Notes
-----

In the process of creating this, I wrote an ascii math parser from scratch.
Parser combinators had some issues differentiating symbols appropriately, and traditional CFGs had trouble with the greedy approach to ascii math (e.g. it'll interpret `sin )` as just the token `sin` with no argument, instead of failing to parse because it doesn't have an argument.
Most javascript CFGs also require specifying the grammar in a text file, which is somewhat obnoxious given the large symbol tables associated with ascii math.

Development
-----------

The extension lives in `extension/`; run these from there.

Compile a local version for development testing
```
bun export
```

Compile a zip for upload to the store
```
bun pack
```

To Do
-----

- [ ] It'd be nice to extend the rendering to multi-line support that works for fixed width fonts.
