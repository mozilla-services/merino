from django.contrib import admin
from polymorphic.admin import (
    PolymorphicParentModelAdmin,
    PolymorphicChildModelAdmin,
)
from .models import (
    ClientVariantSwitchConfig,
    DebugConfig,
    FixedConfig,
    Keyword,
    KeywordFilterConfig,
    MemoryCacheConfig,
    MultiplexerConfig,
    NullProviderConfig,
    ProviderConfig,
    RedisCacheConfig,
    RemoteSettingsConfig,
    StealthConfig,
    TimeoutConfig,
    WikiFruitConfig,
)


@admin.register(ProviderConfig)
class ProviderConfigAdmin(PolymorphicParentModelAdmin):
    base_model = ProviderConfig

    list_display = ("id", "provider_type", "exported_name")

    def get_child_models(self):
        return ProviderConfig.__subclasses__()


class BaseConcreteProviderConfigAdmin(PolymorphicChildModelAdmin):
    list_display = ("id", "provider_type", "exported_name")

    def get_fieldsets(self, request, obj=None):
        return [
            (
                None,
                {
                    "fields": self.get_fields(request, obj=obj),
                    "description": self.base_model.help_text,
                },
            )
        ]


@admin.register(RemoteSettingsConfig)
class RemoteSettingsConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = RemoteSettingsConfig


@admin.register(MemoryCacheConfig)
class MemoryCacheConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = MemoryCacheConfig


@admin.register(RedisCacheConfig)
class RedisCacheConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = RedisCacheConfig


@admin.register(MultiplexerConfig)
class MultiplexerConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = MultiplexerConfig


@admin.register(TimeoutConfig)
class TimeoutConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = TimeoutConfig


@admin.register(FixedConfig)
class FixedConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = FixedConfig


@admin.register(ClientVariantSwitchConfig)
class ClientVariantSwtichConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = ClientVariantSwitchConfig


@admin.register(Keyword)
class KeywordAdmin(admin.ModelAdmin):
    base_model = Keyword
    list_display = ["id", "pattern"]

    def get_fieldsets(self, request, obj=None):
        return [
            (
                None,
                {
                    "fields": self.get_fields(request, obj=obj),
                    "description": self.base_model.help_text,
                },
            )
        ]


class KeywordInlineAdmin(admin.TabularInline):
    model = KeywordFilterConfig.suggestion_blocklist.through
    extra = 0


@admin.register(KeywordFilterConfig)
class KeywordFilterConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = KeywordFilterConfig
    inlines = [KeywordInlineAdmin]
    exclude = ["suggestion_blocklist"]


@admin.register(StealthConfig)
class StealthConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = StealthConfig


@admin.register(DebugConfig)
class DebugConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = DebugConfig


@admin.register(WikiFruitConfig)
class WikiFruitConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = WikiFruitConfig


@admin.register(NullProviderConfig)
class NullProviderConfigAdmin(BaseConcreteProviderConfigAdmin):
    base_model = NullProviderConfig
