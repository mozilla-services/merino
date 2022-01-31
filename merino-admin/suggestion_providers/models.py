import uuid
from typing import Optional

from django.db import models
from polymorphic.models import PolymorphicModel


VALUE_WITH_DEFAULT_HELP_TEXT = """
If left unspecified the Merino server will provide a default value.
"""
INNER_PROVIDER_HELP_TEXT = """
The provider to use to generate suggestions.
"""
DURATION_FORMAT_HELP_TEXT = """
The expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30
seconds. "0.2" would be 200 milliseconds.
"""


class ProviderConfig(PolymorphicModel):
    help_text: Optional[str] = None

    id: models.UUIDField = models.UUIDField(
        primary_key=True, default=uuid.uuid4, editable=False
    )
    exported_name: models.CharField = models.CharField(
        max_length=256,
        blank=True,
        null=True,
        help_text="""
            If this is set, then this provider will be included in the top level
            export to Merino, listed with this name. If this is not set then the
            provider will only be included in the export to Merino if referenced
            by another exported provider
        """,
    )

    def provider_type(self):
        return self.get_real_instance().get_type()

    def get_type(self):
        return "<unknown>"


class RemoteSettingsConfig(ProviderConfig):
    help_text = """
        Load suggestions from a Remote Settings bucket and serve them based on
        the keywords contained in the Remote Settings data.
    """

    bucket: models.CharField = models.CharField(
        max_length=256,
        blank=True,
        null=True,
        help_text="""
            The Remote Settings bucket to load suggestions from. If this is not
            specified, it will default to the global Merino default.
        """,
    )
    collection: models.CharField = models.CharField(
        max_length=256,
        blank=True,
        null=True,
        help_text="""
            The Remote Settings collection to load suggestions from. If this is
            not specified, it will default to the global Merino default.
        """,
    )
    resync_interval: models.DurationField = models.DurationField(
        blank=True,
        null=True,
        help_text=f"""
            The time between re-syncs of Remote Settings data.
            {DURATION_FORMAT_HELP_TEXT} {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    suggestion_score: models.FloatField = models.FloatField(
        blank=True,
        null=True,
        help_text=f"""
            The score to value to assign to suggestions. A float between 0.0 and
            1.0 inclusive. {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )

    def get_type(self):
        return "remote_settings"


class MemoryCacheConfig(ProviderConfig):
    help_text = """
        Cache suggestions in a node-local memory store.
    """

    default_ttl: models.DurationField = models.DurationField(
        blank=True,
        null=True,
        help_text=f"""
            The default TTL to assign to a cache entry if the underlying
            provider does not provide one. {DURATION_FORMAT_HELP_TEXT} {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    cleanup_interval: models.DurationField = models.DurationField(
        blank=True,
        null=True,
        help_text=f"""
            The cleanup task will be run with a period equal to this setting.
            Any expired entries will be removed from the cache.
            {DURATION_FORMAT_HELP_TEXT} {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    max_removed_entries: models.IntegerField = models.IntegerField(
        blank=True,
        null=True,
        help_text=f"""
            While running the cleanup task, at most this many entries will be
            removed before cancelling the task. This should be used to limit the
            maximum amount of time the cleanup task takes.
            {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    default_lock_timeout: models.DurationField = models.DurationField(
        blank=True,
        null=True,
        help_text=f"""
            The default TTL for in-memory locks to prevent multiple update
            requests from being fired at providers at the same time.
            {DURATION_FORMAT_HELP_TEXT} {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    inner: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="memory_cache",
        help_text=INNER_PROVIDER_HELP_TEXT,
    )

    def get_type(self):
        return "memory_cache"


class RedisCacheConfig(ProviderConfig):
    help_text = """
        Cache suggestions in a cluster-wide Redis store.
    """

    default_ttl: models.DurationField = models.DurationField(
        blank=True,
        null=True,
        help_text=f"""
            The default TTL to assign to a cache entry if the underlying
            provider does not provide one. {DURATION_FORMAT_HELP_TEXT}
            {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    default_lock_timeout: models.DurationField = models.DurationField(
        blank=True,
        null=True,
        help_text=f"""
            The default TTL for in-memory locks to prevent multiple update
            requests from being fired at providers at the same time.
            {DURATION_FORMAT_HELP_TEXT} {VALUE_WITH_DEFAULT_HELP_TEXT}
        """,
    )
    inner: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="redis_cache",
        help_text=INNER_PROVIDER_HELP_TEXT,
    )

    def get_type(self):
        return "redis_cache"


class MultiplexerConfig(ProviderConfig):
    help_text = """
        Combine suggestions from multiple providers.
    """

    providers: models.ManyToManyField = models.ManyToManyField(
        ProviderConfig,
        related_name="multiplexer",
        help_text="""
            The providers to generate suggestions with. All providers will be
            run in parallel for all requests.
        """,
    )

    def get_type(self):
        return "multiplexer"


class TimeoutConfig(ProviderConfig):
    help_text = """
        Wait a configurable amount of time for suggestions, and if the inner
        provider takes too long, return an empty result.
    """

    max_time: models.DurationField = models.DurationField(
        help_text=f"""
        After this much time, an empty response will be returned and the request
        to the inner provider will be cancelled. {DURATION_FORMAT_HELP_TEXT}
    """
    )
    inner: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="timeout",
        help_text=INNER_PROVIDER_HELP_TEXT,
    )

    def get_type(self):
        return "timeout"


class FixedConfig(ProviderConfig):
    help_text = """
        A debug provider that always return a suggestion with dummy values and
        configurable fixed string as a title.
    """

    value: models.CharField = models.CharField(
        max_length=256,
        help_text="The value to use for the fixed title of the suggestion.",
    )

    def get_type(self):
        return "fixed"


class Keyword(models.Model):
    help_text = """
        A keyword that can be used to match suggestions for filtering.
    """

    id: models.SlugField = models.SlugField(
        max_length=256,
        primary_key=True,
        help_text="""
            The ID used to identify this pattern in logging and metrics.
        """,
    )
    pattern: models.CharField = models.CharField(
        max_length=1024,
        help_text="""
            If this regex pattern is found in a string, it will match this keyword.
        """,
    )

    def __str__(self):
        return self.id


class KeywordFilterConfig(ProviderConfig):
    help_text = """
        Filter outgoing suggestions based on a configurable block-list of
        patterns. The patterns are applied to the titles of suggestions, and any
        suggestions that match the patterns are removed from the response.
    """

    suggestion_blocklist: models.ManyToManyField = models.ManyToManyField(
        Keyword,
        help_text="""
        If any of these keywords match the title of a suggestion, that
        suggestion will be blocked.
    """,
    )
    inner: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="keyword_filter",
        help_text=INNER_PROVIDER_HELP_TEXT,
    )

    def get_type(self):
        return "keyword_filter"


class ClientVariantSwitchConfig(ProviderConfig):
    help_text = """
        Provider switches between two providers based on whether a request's 
        client_variants matches the configured client_variant string. If there 
        is a match, suggestions will be given from the matching provider. If not,
        the default provider will be used.
    """
    client_variant: models.CharField = models.CharField(
        max_length=256,
        help_text="""
            If this string is found in a client_variants, the matching_provider 
            will be used for suggestions. If not, the default_provider will be 
            used.
        """,
    )
    matching_provider: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="client_variant_switch_matching_provider",
        help_text="""
            The provider to use to generate suggestions when one of the 
            client_variants from a request matches the client_variant field
        """,
    )

    default_provider: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="client_variant_switch_default_provider",
        help_text="""
            The provider to use to generate suggestions when none of the 
            client_variants from a request matches the client_variant field
        """,
    )

    def get_type(self):
        return "client_variant_switch"


class StealthConfig(ProviderConfig):
    help_text = """
        A testing tool that generates the server load that would be associated
        with handling suggestion requests, but silently drops all of the
        generated suggestions.
    """

    inner: models.ForeignKey = models.ForeignKey(
        ProviderConfig,
        on_delete=models.SET_NULL,
        null=True,
        related_name="stealth",
        help_text=f"""
            {INNER_PROVIDER_HELP_TEXT} The suggestion generated will not be sent
            in the response.
        """,
    )

    def get_type(self):
        return "stealth"


class DebugConfig(ProviderConfig):
    help_text = """
        A debugging provider that returns a dummy suggestion with a title that
        contains a serialized version of the suggestion request. This allows
        manual testing of what Merino is seeing about the incoming requests.
    """

    def get_type(self):
        return "debug"


class WikiFruitConfig(ProviderConfig):
    help_text = """
        A debugging provider that has dummy suggestions for three Wikipedia
        pages: Apple, Banana, and Cherry.
    """

    def get_type(self):
        return "wiki_fruit"


class NullProviderConfig(ProviderConfig):
    help_text = """
        A provider that never returns any suggestions.
    """

    def get_type(self):
        return "null"
