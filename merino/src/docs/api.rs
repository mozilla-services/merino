/*!
# Merino API documentation

This page describes the API end points available on Merino.

## Suggest

The primary endpoint for the browser to consume, this endpoint consumes user
input and suggests pages the user may want to visit. The expectation is that
this is shown alongside other content the browser suggests to the user, such as
bookmarks and history.

This endpoint accepts GET requests and takes parameters as query string values
and headers.

### Query Parameters

- `q` - The query that the user has typed. This is expected to be a partial
  input, sent as fast as once per keystroke, though a slower period may be
  appropriate for the user agent.

### Headers

- `Accept-Language` - The language preferences expressed in this header will be
  honored. Merino will attempt to delivery suggestions in the preferred
  languages. If none are available, or if Merino does not recognize the
  languages specified, no results will be returned. Merino will prefer to give
  suggestions in a language with a lower quality value over giving no responses.
  Languages with `q=0` or not mentioned in the header will not produce any
  suggestions.

### Other derived language

- Location - The IP address of the user or nearest proxy will be used to
  determine location. This location may be as granular as city level, depending
  on server configuration.

### Response

The response will be a JSON object containing a single key, `suggestions`, which
will be a list of suggestion objects. Each suggestion object will have the
following keys:

- `block_id` - a number that can be used, along with the `provider` field below, to
  uniquely identify this suggestion. Two suggestions with the same `provider`
  and `block_id` may not be byte-for-byte identical, but will be semantically
  equivalent.

- `full_keyword` - In the case that the query was a partial match to the
  suggestion, this is the completed query that would also match this query. For
  example, if the user was searching for fruit and typed "appl", this field
  might contain the string "apples". This is be suitable to show as a completion
  of the user's input. This field should be treated as plain text.

- `title` - The full title of the suggestion resulting from the query. Using the
  example of apples above, this might be "Types of Apples in the Pacific
  Northwest". This field should be treated as plain text.

- `url` - The URL of the page that should be navigated to if the user selects
  this suggestion. This will will be a resource with the title specified in the
  `title` field.

- `impression_url` - A telemetry URL that should be notified if the browser
  shows this suggestion to the user. This is used along with `click_url` to
  monitor the relevancy of suggestions. For more details see Interaction Pings,
  below.

- `click_url` - A telemetry URL that should be notified if the user selects this
  suggestion. This should only be notified as the result of positive user
  action, and only if the user has navigated to the page specified in the `url`
  field. For more details see Interaction Pings, below.

- `provider` - A string that identifies the source of this suggestion. This can
  be used along with `block_id` to uniquely identify this suggestion. It is not
  intended to be directly displayed to the user.

- ~~`advertiser`~~ - A deprecated alias of `provider`.

- `is_sponsored` - A boolean indicating if this suggestion is sponsored content.
  If this is true, the UI must indicate to the user that the suggestion is
  sponsored.

- `icon` - A URL of an image to display alongside the suggestion. This will be a
  small square image, suitable to be included inline with the text, such as a
  site's favicon.

### Response Headers

Responses will carry standard HTTP caching headers that indicate the validity of
the suggestions. User agents should prefer to provide the user with cached
results as indicated by these headers.

### Response Status Codes

- 200 OK - Suggestions provided normally.
- 4xx - Client error. See response for details.
- 5xx - Internal server error. Try again later.

## Interaction Pings

In order to indicate activity relating to a suggestion, a ping should be sent to
the provided URLs, either the click URL or the impression URL, as appropriate.
These pings should be delegated to a Mozilla-controlled service instead of being
sent directly by the browser. This is preferable since it helps maintain user
privacy.

Requests to the interaction URL should TBD.

*/
