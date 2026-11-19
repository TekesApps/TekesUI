# Inspector key/value presentation

Issue: https://github.com/TekesApps/TekesUI/issues/310

The shared `KeyValueTableContent` owns two aligned columns, common row padding,
wrapped row measurement and token-based separators. It accepts pairs of ordinary
Content children: it does not own an inspection model, create text owners, or add
sorting, selection or virtualization. Keys receive one third of the usable width;
values receive the remainder. Both cells share the measured height of their row.
The existing SeparatorView owns separator color and backing-scale policy.
After arrangement the container declares the existing Host reveal passthrough
contract: scrolling translates its ordinary child frames with no local clip or
alternate owner. This is required for prepared pointer focus of selectable values;
measurement and lifecycle changes clear that declaration until rearranged.

GridContent only provides equal-width cells and spacing. DataTableContent rejects
interactive cell displays, so it cannot preserve Inspector's read-only selectable
TextFieldContent owners. A small dedicated layout container avoids altering those
existing contracts or adding consumer-specific drawing.

Gallery retains group headings, snapshot extraction, scroll ownership and original
TextFieldContent children. Same-schema publication updates the existing value IDs;
instance changes retain the existing replacement rule. No changes to inspection,
text editing/copy, or external-value contracts are required.

Verification: shared CPU geometry checks at narrow/wide widths with wrapped values,
plus the existing Gallery Inspector selection/copy/read-only and mount-retention
checks. Native light/dark readability and long-value copy remain separate acceptance
checks; source delivery does not claim native or release completion.
