# Generated by Django 4.0.2 on 2022-02-01 10:54

from django.db import migrations, models
import django.db.models.deletion
import uuid


class Migration(migrations.Migration):

    initial = True

    dependencies = [
        ('contenttypes', '0002_remove_content_type_name'),
    ]

    operations = [
        migrations.CreateModel(
            name='Keyword',
            fields=[
                ('id', models.SlugField(help_text='\n            The ID used to identify this pattern in logging and metrics.\n        ', max_length=256, primary_key=True, serialize=False)),
                ('pattern', models.CharField(help_text='\n            If this regex pattern is found in a string, it will match this keyword.\n        ', max_length=1024)),
            ],
        ),
        migrations.CreateModel(
            name='ProviderConfig',
            fields=[
                ('id', models.UUIDField(default=uuid.uuid4, editable=False, primary_key=True, serialize=False)),
                ('exported_name', models.CharField(blank=True, help_text='\n            If this is set, then this provider will be included in the top level\n            export to Merino, listed with this name. If this is not set then the\n            provider will only be included in the export to Merino if referenced\n            by another exported provider\n        ', max_length=256, null=True)),
                ('polymorphic_ctype', models.ForeignKey(editable=False, null=True, on_delete=django.db.models.deletion.CASCADE, related_name='polymorphic_%(app_label)s.%(class)s_set+', to='contenttypes.contenttype')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
        ),
        migrations.CreateModel(
            name='DebugConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='FixedConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('value', models.CharField(help_text='The value to use for the fixed title of the suggestion.', max_length=256)),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='NullProviderConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='RemoteSettingsConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('bucket', models.CharField(blank=True, help_text='\n            The Remote Settings bucket to load suggestions from. If this is not\n            specified, it will default to the global Merino default.\n        ', max_length=256, null=True)),
                ('collection', models.CharField(blank=True, help_text='\n            The Remote Settings collection to load suggestions from. If this is\n            not specified, it will default to the global Merino default.\n        ', max_length=256, null=True)),
                ('resync_interval', models.DurationField(blank=True, help_text='\n            The time between re-syncs of Remote Settings data.\n            \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('suggestion_score', models.FloatField(blank=True, help_text='\n            The score to value to assign to suggestions. A float between 0.0 and\n            1.0 inclusive. \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='WikiFruitConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='TimeoutConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('max_time', models.DurationField(help_text='\n        After this much time, an empty response will be returned and the request\n        to the inner provider will be cancelled. \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n\n    ')),
                ('inner', models.ForeignKey(help_text='\nThe provider to use to generate suggestions.\n', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='timeout', to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='StealthConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('inner', models.ForeignKey(help_text='\n            \nThe provider to use to generate suggestions.\n The suggestion generated will not be sent\n            in the response.\n        ', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='stealth', to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='RedisCacheConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('default_ttl', models.DurationField(blank=True, help_text='\n            The default TTL to assign to a cache entry if the underlying\n            provider does not provide one. \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n\n            \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('default_lock_timeout', models.DurationField(blank=True, help_text='\n            The default TTL for in-memory locks to prevent multiple update\n            requests from being fired at providers at the same time.\n            \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('inner', models.ForeignKey(help_text='\nThe provider to use to generate suggestions.\n', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='redis_cache', to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='MultiplexerConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('providers', models.ManyToManyField(help_text='\n            The providers to generate suggestions with. All providers will be\n            run in parallel for all requests.\n        ', related_name='multiplexer', to='merino_suggestion_providers.ProviderConfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='MemoryCacheConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('default_ttl', models.DurationField(blank=True, help_text='\n            The default TTL to assign to a cache entry if the underlying\n            provider does not provide one. \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('cleanup_interval', models.DurationField(blank=True, help_text='\n            The cleanup task will be run with a period equal to this setting.\n            Any expired entries will be removed from the cache.\n            \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('max_removed_entries', models.IntegerField(blank=True, help_text='\n            While running the cleanup task, at most this many entries will be\n            removed before cancelling the task. This should be used to limit the\n            maximum amount of time the cleanup task takes.\n            \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('default_lock_timeout', models.DurationField(blank=True, help_text='\n            The default TTL for in-memory locks to prevent multiple update\n            requests from being fired at providers at the same time.\n            \nThe expected format is HH:MM:SS.dd. Example, "1:30" would be 1 minute and 30\nseconds. "0.2" would be 200 milliseconds.\n \nIf left unspecified the Merino server will provide a default value.\n\n        ', null=True)),
                ('inner', models.ForeignKey(help_text='\nThe provider to use to generate suggestions.\n', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='memory_cache', to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='KeywordFilterConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('inner', models.ForeignKey(help_text='\nThe provider to use to generate suggestions.\n', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='keyword_filter', to='merino_suggestion_providers.providerconfig')),
                ('suggestion_blocklist', models.ManyToManyField(help_text='\n        If any of these keywords match the title of a suggestion, that\n        suggestion will be blocked.\n    ', to='merino_suggestion_providers.Keyword')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
        migrations.CreateModel(
            name='ClientVariantSwitchConfig',
            fields=[
                ('providerconfig_ptr', models.OneToOneField(auto_created=True, on_delete=django.db.models.deletion.CASCADE, parent_link=True, primary_key=True, serialize=False, to='merino_suggestion_providers.providerconfig')),
                ('client_variant', models.CharField(help_text='\n            If this string is found in a client_variants, the matching_provider\n            will be used for suggestions. If not, the default_provider will be\n            used.\n        ', max_length=256)),
                ('default_provider', models.ForeignKey(help_text='\n            The provider to use to generate suggestions when none of the\n            client_variants from a request matches the client_variant field\n        ', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='client_variant_switch_default_provider', to='merino_suggestion_providers.providerconfig')),
                ('matching_provider', models.ForeignKey(help_text='\n            The provider to use to generate suggestions when one of the\n            client_variants from a request matches the client_variant field\n        ', null=True, on_delete=django.db.models.deletion.SET_NULL, related_name='client_variant_switch_matching_provider', to='merino_suggestion_providers.providerconfig')),
            ],
            options={
                'abstract': False,
                'base_manager_name': 'objects',
            },
            bases=('merino_suggestion_providers.providerconfig',),
        ),
    ]
