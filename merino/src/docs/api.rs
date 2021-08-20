/*!
# Merino API documentation

This page describes the API endpoints available on Merino.

## Suggest

Endpoint: `/api/v1/suggest`

Example: `/api/v1/suggest?q=nelson%20mand`

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

- `Accept-Language` - The locale preferences expressed in this header in
  accordance with [RFC 2616 section 14.4][rfc-2616-14-4] will be used to determine suggestions.
  Merino maintains a list of supported locales. Merino will choose the
  locale from it's list that has the highest `q` (quality) value in the
  user's `Accept-Language` header. Locales with `q=0` will not be used.

  If no locales match, Merino will not return any suggestions. If the
  header is not included or empty, Merino will default to the `en-US` locale.

  If the highest quality, compatible language produces no suggestion results,
  Merino will return an empty list instead of attempting to query other
  languages.

[rfc-2616-14-4]: https://datatracker.ietf.org/doc/html/rfc2616/#section-14.4

### Other derived inputs

- Location - The IP address of the user or nearest proxy will be used to
  determine location. This location may be as granular as city level, depending
  on server configuration.

  Users that use VPN services will be identified according to the VPN exit
  node they use, allowing them to change Merino's understanding of their
  location. VPN exit nodes are often mis-identified in geolocation databases,
  and may produce unreliable results.

### Response

The response will be a JSON object containing a single key, `suggestions`, which
will be a list of suggestion objects. Each suggestion object will have the
following keys:

- `block_id` - a number that can be used, along with the `provider` field below, to
  uniquely identify this suggestion. Two suggestions with the same `provider`
  and `block_id` should be treated as the same suggestion, even if other
  fields, such as `click_url` change. Merino will enforce that they are
  equivalent from a user's point of view.

- `full_keyword` - In the case that the query was a partial match to the
  suggestion, this is the completed query that would also match this query. For
  example, if the user was searching for fruit and typed "appl", this field
  might contain the string "apples". This is suitable to show as a completion
  of the user's input. This field should be treated as plain text.

- `title` - The full title of the suggestion resulting from the query. Using the
  example of apples above, this might be "Types of Apples in the Pacific
  Northwest". This field should be treated as plain text.

- `url` - The URL of the page that should be navigated to if the user selects
  this suggestion. This will be a resource with the title specified in the
  `title` field.

- `impression_url` - A provider specified telemetry URL that should be notified
  if the browser shows this suggestion to the user. This is used along with
  `click_url` to monitor the relevancy of suggestions. For more details see
  [Interaction Pings](#interaction-pings), below. This field may be null, in which
  case no impression ping is required for this suggestion provider.

- `click_url` - A provider specified telemetry URL that should be notified
  if the user selects this suggestion. This should only be notified as the result
  of positive user action, and only if the user has navigated to the page specified
  in the `url` field. For more details see [Interaction Pings](#interaction-pings),
  below. This field may be null, in which case no click ping is required for this
  suggestion provider.

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

<a id="interaction-pings"></a>
## Interaction Pings

When a Firefox user views or selects a suggestion from Merino, Firefox will
send an impression or a click ping to a Mozilla-controlled service indicating
this user interaction. Some suggestion providers may also need that interaction
data for reporting and relevancy optimization. Firefox will not send the pings
to those providers directly, rather, it will delegate those to a Mozilla-controlled
service, by which the interaction pings will be sent to the `impression_url` or
`click_url` specified by the providers.

If the URL for an interaction ping is not specified (for example, `click_url`
is `null`), then no ping should be sent to the provider for that action. However,
this interaction ping is always sent to the Mozilla-controlled service unless
the user opts out the telemetry collection of Firefox.

The required behavior for interaction pings is TBD.

*/
