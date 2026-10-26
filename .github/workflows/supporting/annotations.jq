def sanitize:
  tostring | gsub("%"; "%25") | gsub("\n"; "%0A") | gsub("\r"; "%0D");
def property:
  sanitize | gsub(","; "%2C") | gsub(":"; "%3A");
def level:
  if . == "help" or . == "note" then "notice"
  elif . == "warning" then "warning" else "error" end;
.[]
| select(.reason == "compiler-message")
| .message
| . as $message
| .spans[] | select(.is_primary)
| "::\($message.level | level) "
  + "file=\(($WORKING_DIRECTORY + "/" + (.file_name | gsub("\\\\"; "/"))) | property),"
  + "line=\(.line_start),endLine=\(.line_end),col=\(.column_start),endColumn=\(.column_end),"
  + "title=\($message.message | property)::\(($message.rendered // $message.message) | sanitize)"
