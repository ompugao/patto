import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'src/app.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // TODO: Initialize Rust library when flutter_rust_bridge is set up
  // await RustLib.init();

  runApp(
    const ProviderScope(
      child: PattoApp(),
    ),
  );
}
