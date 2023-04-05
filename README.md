# modify-crc32

Modify CRC32 hash of a file.

<table width="100%">
  <thead>
    <tr>
      <th width="50%">GUI</th>
      <th width="50%">CLI</th>
    </tr>
  </thead>
  <tbody align="center">
    <tr>
      <td width="50%">
        <img src="https://user-images.githubusercontent.com/99479536/229995470-b9654beb-1d8b-4d9b-a0ce-e2ca781444aa.gif"/>
      </td>
      <td width="50%">
        <a href="https://asciinema.org/a/soayvMFqa7GTjUILd2YxjvGMG">
          <img src="https://asciinema.org/a/soayvMFqa7GTjUILd2YxjvGMG.svg"/>
        </a>
      </td>
    </tr>
  </tbody>
</table>

## CLI Usage

```
Usage: modify-crc32 <filename> <new_crc32> [-x]

Modify CRC32 of a file

Positional Arguments:
  filename          the file to modify
  new_crc32         the CRC32 hash to modify to

Options:
  -x, --execute     don't prompt when patching the file
  --help            display usage information
```
