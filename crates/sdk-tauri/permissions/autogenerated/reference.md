## Default Permission

Allows rendering a PrintDocument through core-rs (all render_* commands plus validate_document). This plugin has no delivery transport of its own -- see crates/sdk-tauri/src/lib.rs -- so granting this permission only lets a window ask for bytes/HTML/text back, never talk to a printer directly.

#### This default permission set includes the following:

- `allow-render-to-escpos`
- `allow-render-to-html`
- `allow-render-to-svg`
- `allow-render-to-plain-text`
- `allow-render-to-pdf`
- `allow-validate-document`

## Permission Table

<table>
<tr>
<th>Identifier</th>
<th>Description</th>
</tr>


<tr>
<td>

`printers-sdk-tauri:allow-render-to-escpos`

</td>
<td>

Enables the render_to_escpos command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:deny-render-to-escpos`

</td>
<td>

Denies the render_to_escpos command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:allow-render-to-html`

</td>
<td>

Enables the render_to_html command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:deny-render-to-html`

</td>
<td>

Denies the render_to_html command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:allow-render-to-pdf`

</td>
<td>

Enables the render_to_pdf command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:deny-render-to-pdf`

</td>
<td>

Denies the render_to_pdf command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:allow-render-to-plain-text`

</td>
<td>

Enables the render_to_plain_text command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:deny-render-to-plain-text`

</td>
<td>

Denies the render_to_plain_text command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:allow-render-to-svg`

</td>
<td>

Enables the render_to_svg command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:deny-render-to-svg`

</td>
<td>

Denies the render_to_svg command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:allow-validate-document`

</td>
<td>

Enables the validate_document command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`printers-sdk-tauri:deny-validate-document`

</td>
<td>

Denies the validate_document command without any pre-configured scope.

</td>
</tr>
</table>
