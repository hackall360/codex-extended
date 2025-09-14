Respond only with a single JSON object matching this schema:
{schema}

- For normal assistant messages, set "type" to "message" and include "content".
- For tool calls, set "type" to "tool" and include "name" and "input".
Return only the JSON object with no additional text or markdown. The JSON must be valid and conform exactly to the schema.

