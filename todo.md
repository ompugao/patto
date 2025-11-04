* Todo:
  * Parser
    * [x] scan file in subdirectories
    * [x] handle link to localfile
    * [x] handle inline math \(math text \) 
    * [x] rendering inline math
    * [x] handle [XXX](mailto:XXX@example.com)  
    * [x] support abbrev task, like `!2024-09-24T13:00 `
      * [ ] better abbrev, add `pending` status
    * [x] better anchor handling `name#anchor`. some note contains `#` in its name.
      * we do not support `#` for the name of notes.
    * eliminate the logic that self-link if link is empty
    * better depth and state handling
  * LSP server
    * [x] async note scanning
    * [x] return all errors as diagnostics
    * [x] goto definition for hopping between notes
      * [x] support both [[note]] and [[note#anchor]]
    * semantic tokens
      * `overlappingTokenSupport` seems not supported broadly (only neovim and vscode, AFAIK)
    * [x] support note and anchor completion
    * [x] document references and anchor references
      * [x] directional graph construction
    * [x] find references (backlinks) from other notes
    * [x] export markdown
    * todo extraction
      * [x] aggregation command
      * [x] vim-lsp version
      * [x] nvim-lspconfig version
      * [x] vscode version
      * auto refresh
    * [ ] note renaming
      * vim-lsp does not support CreateFile/RenameFile/DeleteFile
        * [https://github.com/prabirshrestha/vim-lsp/issues/1371](https://github.com/prabirshrestha/vim-lsp/issues/1371)
      * yegappan/lsp supports these
    * [ ] anchor renaming
    * [ ] make error.variant.message() user-friendly
    * [ ] fix indentation error at a line after a block with trailing empty lines
    * [ ] lsp server hangs sometimes
  * Previewer
    * [x] realtime preview
    * [x] bugfix: id jump (including self-link)
    * [x] bugfix: set page title using note's name
    * [x] bugfix: history handling
    * [x] feature: support sidebar folding
    * [x] feature: support mermaid
    * [x] feature: support printing
    * [x] bugfix: Fix twitter embedding css
    * [x] back links & show two hop links
