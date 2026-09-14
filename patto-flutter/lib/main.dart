import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import 'app.dart';
import 'src/rust/frb_api.dart' as rust;
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await _installCaBundle();
  runApp(const ProviderScope(child: PattoApp()));
}

/// The bundled OpenSSL has no trust store, so libgit2 cannot verify any HTTPS
/// certificate until it is given a CA bundle. Copy the asset to a real file and
/// point libgit2 at it.
Future<void> _installCaBundle() async {
  try {
    final dir = await getApplicationSupportDirectory();
    final file = File('${dir.path}/cacert.pem');
    if (!await file.exists()) {
      final data = await rootBundle.load('assets/cacert.pem');
      await file.writeAsBytes(data.buffer.asUint8List(), flush: true);
    }
    await rust.gitInitRuntime(caBundlePath: file.path);
  } catch (e) {
    debugPrint('Could not install the CA bundle: $e');
  }
}
