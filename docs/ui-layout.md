# Configuration layout

The editor has five directly accessible sections: Keypad, Layers & thumb, Lighting, Profiles and App settings. The runtime Start/Stop controls and status remain in the application header. Mapping Apply/Reload actions are hidden in Lighting and App settings, whose controls save independently.

Keypad shows the physical layout beside the selected control's assignment on larger windows. The board scales to the available width and height. Narrow windows show the board or the assignment editor, with a Back to keypad button and a Choose control selector for precise selection. Live input diagnostics and D-pad notes are available in expandable sections under the board.

Macro sequence editing expands to the full workspace. Numbered cards run left to right with Add step at the end. Long sequences scroll horizontally within the track; unusually short windows can also scroll inside that track. Reordering uses Move left/right. Opening other sections preserves the current form values; applying an assignment and saving mappings retain their existing explicit steps.

Layers & thumb groups layer creation, naming and thumb switching. Profiles separates profile selection/saving from import/export. App settings groups startup options, connection information and appearance diagnostics. Tabs support arrow keys, Home and End, and expose selected state and panel associations to assistive technology.

This change reorganizes existing frontend controls and retains their IDs, handlers, API calls and configuration formats. `web/layout.js` owns navigation and fitting, using resize notifications rather than polling. Input capture, injection, macro playback, RGB protocol, drivers and startup behavior are unchanged. Small windows or long lists can still require local scrolling; the main document stays within the window.

## Validation

Browser checks covered all section destinations, narrow-window key selection and return, layer selection across sections, macro card reordering/add/remove, and retention of an edited pause after navigation. At 1440×900, 1024×650, 560×720 and 360×640, document dimensions matched the viewport, the sequence editor remained visible, and step cards stayed in one horizontal row. A control selector avoids relying on tiny board buttons at the smallest size. Very short windows can require vertical scrolling inside the sequence track. Browser error logs were empty after final checks. JavaScript syntax, formatting, strict Clippy and the optimized build passed. Settings file hashes for mappings, launch settings and lighting matched their pre-edit values. Evidence is in `artifacts/ui-layout/`.
