import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_flutter/webview_flutter.dart';

import '../../../core/embed_metadata.dart';
import '../../../src/rust/api/types.dart';

/// Full-screen web view for an embed, the way the web preview frames it.
///
/// A tweet is rendered from its oEmbed snippet with Twitter's widget script;
/// a deck opens the site's own player page; anything else is the URL itself.
/// The lookups are the cached ones the card already made, so opening is
/// usually immediate, and a failed lookup falls back to loading the URL.
class EmbedViewerScreen extends StatefulWidget {
  const EmbedViewerScreen({super.key, required this.embed});

  final EmbedRef embed;

  static void open(BuildContext context, EmbedRef embed) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => EmbedViewerScreen(embed: embed)),
    );
  }

  @override
  State<EmbedViewerScreen> createState() => _EmbedViewerScreenState();
}

class _EmbedViewerScreenState extends State<EmbedViewerScreen> {
  late final WebViewController _controller;
  int _progress = 0;
  String? _lookedUpTitle;
  String? _error;
  bool _canGoBack = false;

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(
        NavigationDelegate(
          onProgress: (progress) {
            if (mounted) setState(() => _progress = progress);
          },
          onPageFinished: (_) => _updateCanGoBack(),
          onNavigationRequest: _onNavigationRequest,
        ),
      );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _controller.setBackgroundColor(Theme.of(context).colorScheme.surface);
      _load();
    });
  }

  static const _inlineSchemes = {'http', 'https', 'about', 'data', 'blob'};

  /// Only web pages stay in the view; `twitter://`, `intent://`, `mailto:`
  /// and the like go to the app that handles them, as a browser would.
  /// Frames inside the page (iOS reports them too) are always left alone.
  Future<NavigationDecision> _onNavigationRequest(
    NavigationRequest request,
  ) async {
    if (!request.isMainFrame) return NavigationDecision.navigate;
    final uri = Uri.tryParse(request.url);
    if (uri == null) return NavigationDecision.prevent;
    if (_inlineSchemes.contains(uri.scheme)) {
      return NavigationDecision.navigate;
    }
    await launchUrl(
      uri,
      mode: LaunchMode.externalApplication,
    ).catchError((_) => false);
    return NavigationDecision.prevent;
  }

  Future<void> _updateCanGoBack() async {
    final canGoBack = await _controller.canGoBack();
    if (mounted && canGoBack != _canGoBack) {
      setState(() => _canGoBack = canGoBack);
    }
  }

  Future<void> _load() async {
    final embed = widget.embed;
    final url = Uri.tryParse(embed.url);
    if (url == null || !url.hasScheme) {
      setState(() {
        _error = 'This is not a web address the viewer can open.';
        _progress = 100;
      });
      return;
    }

    switch (embed.kind) {
      case EmbedKind_Twitter():
        final tweet = await Tweets.lookup(embed.url);
        if (!mounted) return;
        if (tweet == null) {
          await _controller.loadRequest(url);
          return;
        }
        setState(() => _lookedUpTitle = tweet.author);
        final dark = Theme.of(context).brightness == Brightness.dark;
        await _controller.loadHtmlString(
          _tweetDocument(tweet.html, dark: dark),
          baseUrl: 'https://twitter.com/',
        );
      case EmbedKind_SpeakerDeck():
        await _loadDeck(await Decks.speakerDeck(embed.url), url);
      case EmbedKind_SlideShare():
        await _loadDeck(await Decks.slideShare(embed.url), url);
      default:
        await _controller.loadRequest(url);
    }
  }

  Future<void> _loadDeck(DeckPreview? deck, Uri url) async {
    if (!mounted) return;
    if (deck?.title != null) setState(() => _lookedUpTitle = deck!.title);
    final player = deck?.playerUrl == null
        ? null
        : Uri.tryParse(deck!.playerUrl!);
    await _controller.loadRequest(player ?? url);
  }

  /// The oEmbed snippet is just a `<blockquote>`; the widget script turns it
  /// into the rendered tweet. Set to the app's theme, since the page behind
  /// it is otherwise white.
  static String _tweetDocument(String html, {required bool dark}) {
    final themed = dark
        ? html.replaceFirst(
            'class="twitter-tweet"',
            'class="twitter-tweet" data-theme="dark"',
          )
        : html;
    final background = dark ? '#121212' : '#ffffff';
    return '''
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  body { margin: 0; padding: 12px; background: $background; }
  .twitter-tweet { margin: 0 auto !important; }
</style>
</head>
<body>
$themed
<script async src="https://platform.twitter.com/widgets.js" charset="utf-8"></script>
</body>
</html>
''';
  }

  Future<void> _openExternally() async {
    final uri = Uri.tryParse(widget.embed.url);
    if (uri == null ||
        !await launchUrl(uri, mode: LaunchMode.externalApplication)) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not open ${widget.embed.url}')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final embed = widget.embed;
    final title =
        embed.title ??
        _lookedUpTitle ??
        Uri.tryParse(embed.url)?.host ??
        embed.url;
    return PopScope(
      canPop: !_canGoBack,
      onPopInvokedWithResult: (didPop, _) async {
        if (didPop) return;
        if (await _controller.canGoBack()) {
          await _controller.goBack();
          await _updateCanGoBack();
        } else if (mounted) {
          setState(() => _canGoBack = false);
        }
      },
      child: _scaffold(context, title),
    );
  }

  Widget _scaffold(BuildContext context, String title) {
    return Scaffold(
      appBar: AppBar(
        title: Text(title, overflow: TextOverflow.ellipsis),
        actions: [
          IconButton(
            icon: const Icon(Icons.open_in_browser),
            tooltip: 'Open in browser',
            onPressed: _openExternally,
          ),
        ],
        bottom: _progress < 100
            ? PreferredSize(
                preferredSize: const Size.fromHeight(2),
                child: LinearProgressIndicator(
                  minHeight: 2,
                  value: _progress / 100,
                ),
              )
            : null,
      ),
      body: _error != null
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text(
                  '$_error\n\n${widget.embed.url}',
                  textAlign: TextAlign.center,
                ),
              ),
            )
          : WebViewWidget(controller: _controller),
    );
  }
}
