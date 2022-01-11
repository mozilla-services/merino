import uuid

from django.db import models
from polymorphic.models import PolymorphicModel


class ProviderConfig(PolymorphicModel):
    id: models.UUIDField = models.UUIDField(
        primary_key=True, default=uuid.uuid4, editable=False
    )
    exported_name: models.CharField = models.CharField(max_length=256, blank=True, null=True)

    def provider_type(self):
        return self.get_real_instance().get_type()

    def get_type(self):
        return '<unknown>'


class RemoteSettingsConfig(ProviderConfig):
    bucket: models.CharField = models.CharField(max_length=256, blank=True, null=True)
    collection: models.CharField = models.CharField(max_length=256, blank=True, null=True)
    resync_interval: models.DurationField = models.DurationField(blank=True, null=True)
    suggestion_score: models.FloatField = models.FloatField(blank=True, null=True)

    def get_type(self):
        return "remote_settings"


class MemoryCacheConfig(ProviderConfig):
    default_ttl: models.DurationField = models.DurationField(blank=True, null=True)
    cleanup_interval: models.DurationField = models.DurationField(blank=True, null=True)
    max_removed_entries: models.IntegerField = models.IntegerField(blank=True, null=True)
    default_lock_timeout: models.DurationField = models.DurationField(blank=True, null=True)
    inner: models.ForeignKey = models.ForeignKey(ProviderConfig, on_delete=models.SET_NULL, null=True, related_name="memory_cache")

    def get_type(self):
        return "memory_cache"


class RedisCacheConfig(ProviderConfig):
    default_ttl: models.DurationField = models.DurationField(blank=True, null=True)
    default_lock_timeout: models.DurationField = models.DurationField(blank=True, null=True)
    inner: models.ForeignKey = models.ForeignKey(ProviderConfig, on_delete=models.SET_NULL, null=True, related_name="redis_cache")

    def get_type(self):
        return "redis_cache"


class MultiplexerConfig(ProviderConfig):
    providers: models.ManyToManyField = models.ManyToManyField(ProviderConfig, related_name='multiplexer')

    def get_type(self):
        return "multiplexer"


class TimeoutConfig(ProviderConfig):
    default_ttl: models.DurationField = models.DurationField()
    inner: models.ForeignKey = models.ForeignKey(ProviderConfig, on_delete=models.SET_NULL, null=True, related_name="timeout")

    def get_type(self):
        return "timeout"


class FixedConfig(ProviderConfig):
    value: models.CharField = models.CharField(max_length=256, )

    def get_type(self):
        return "fixed"


class KeywordFilterConfig(ProviderConfig):
    suggestion_blocklist: models.TextField = models.TextField()
    inner: models.ForeignKey = models.ForeignKey(ProviderConfig, on_delete=models.SET_NULL, null=True, related_name="keyword_filter")

    def get_type(self):
        return "keyword_filter"


class StealthConfig(ProviderConfig):
    inner: models.ForeignKey = models.ForeignKey(ProviderConfig, on_delete=models.SET_NULL, null=True, related_name="stealth")

    def get_type(self):
        return "stealth"


class DebugConfig(ProviderConfig):
    def get_type(self):
        return "debug"


class WikiFruitConfig(ProviderConfig):
    def get_type(self):
        return "wiki_fruit"


class NullProviderConfig(ProviderConfig):
    def get_type(self):
        return "null"
