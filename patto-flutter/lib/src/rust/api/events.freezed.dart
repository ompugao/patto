// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'events.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$CloneEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CloneEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CloneEvent()';
}


}

/// @nodoc
class $CloneEventCopyWith<$Res>  {
$CloneEventCopyWith(CloneEvent _, $Res Function(CloneEvent) __);
}


/// Adds pattern-matching-related methods to [CloneEvent].
extension CloneEventPatterns on CloneEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CloneEvent_Progress value)?  progress,TResult Function( CloneEvent_Done value)?  done,TResult Function( CloneEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CloneEvent_Progress() when progress != null:
return progress(_that);case CloneEvent_Done() when done != null:
return done(_that);case CloneEvent_Failed() when failed != null:
return failed(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CloneEvent_Progress value)  progress,required TResult Function( CloneEvent_Done value)  done,required TResult Function( CloneEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case CloneEvent_Progress():
return progress(_that);case CloneEvent_Done():
return done(_that);case CloneEvent_Failed():
return failed(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CloneEvent_Progress value)?  progress,TResult? Function( CloneEvent_Done value)?  done,TResult? Function( CloneEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case CloneEvent_Progress() when progress != null:
return progress(_that);case CloneEvent_Done() when done != null:
return done(_that);case CloneEvent_Failed() when failed != null:
return failed(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( GitProgress progress)?  progress,TResult Function()?  done,TResult Function( Failure failure)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CloneEvent_Progress() when progress != null:
return progress(_that.progress);case CloneEvent_Done() when done != null:
return done();case CloneEvent_Failed() when failed != null:
return failed(_that.failure);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( GitProgress progress)  progress,required TResult Function()  done,required TResult Function( Failure failure)  failed,}) {final _that = this;
switch (_that) {
case CloneEvent_Progress():
return progress(_that.progress);case CloneEvent_Done():
return done();case CloneEvent_Failed():
return failed(_that.failure);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( GitProgress progress)?  progress,TResult? Function()?  done,TResult? Function( Failure failure)?  failed,}) {final _that = this;
switch (_that) {
case CloneEvent_Progress() when progress != null:
return progress(_that.progress);case CloneEvent_Done() when done != null:
return done();case CloneEvent_Failed() when failed != null:
return failed(_that.failure);case _:
  return null;

}
}

}

/// @nodoc


class CloneEvent_Progress extends CloneEvent {
  const CloneEvent_Progress({required this.progress}): super._();
  

 final  GitProgress progress;

/// Create a copy of CloneEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CloneEvent_ProgressCopyWith<CloneEvent_Progress> get copyWith => _$CloneEvent_ProgressCopyWithImpl<CloneEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CloneEvent_Progress&&(identical(other.progress, progress) || other.progress == progress));
}


@override
int get hashCode {
    return Object.hash(runtimeType,progress);
}

@override
String toString() {
    return 'CloneEvent.progress(progress: $progress)';
}


}

/// @nodoc
abstract mixin class $CloneEvent_ProgressCopyWith<$Res> implements $CloneEventCopyWith<$Res> {
  factory $CloneEvent_ProgressCopyWith(CloneEvent_Progress value, $Res Function(CloneEvent_Progress) _then) = _$CloneEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 GitProgress progress
});




}
/// @nodoc
class _$CloneEvent_ProgressCopyWithImpl<$Res>
    implements $CloneEvent_ProgressCopyWith<$Res> {
  _$CloneEvent_ProgressCopyWithImpl(this._self, this._then);

  final CloneEvent_Progress _self;
  final $Res Function(CloneEvent_Progress) _then;

/// Create a copy of CloneEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? progress = null,}) {
  return _then(CloneEvent_Progress(
progress: null == progress ? _self.progress : progress // ignore: cast_nullable_to_non_nullable
as GitProgress,
  ));
}


}

/// @nodoc


class CloneEvent_Done extends CloneEvent {
  const CloneEvent_Done(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CloneEvent_Done);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'CloneEvent.done()';
}


}




/// @nodoc


class CloneEvent_Failed extends CloneEvent {
  const CloneEvent_Failed({required this.failure}): super._();
  

 final  Failure failure;

/// Create a copy of CloneEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CloneEvent_FailedCopyWith<CloneEvent_Failed> get copyWith => _$CloneEvent_FailedCopyWithImpl<CloneEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is CloneEvent_Failed&&(identical(other.failure, failure) || other.failure == failure));
}


@override
int get hashCode {
    return Object.hash(runtimeType,failure);
}

@override
String toString() {
    return 'CloneEvent.failed(failure: $failure)';
}


}

/// @nodoc
abstract mixin class $CloneEvent_FailedCopyWith<$Res> implements $CloneEventCopyWith<$Res> {
  factory $CloneEvent_FailedCopyWith(CloneEvent_Failed value, $Res Function(CloneEvent_Failed) _then) = _$CloneEvent_FailedCopyWithImpl;
@useResult
$Res call({
 Failure failure
});




}
/// @nodoc
class _$CloneEvent_FailedCopyWithImpl<$Res>
    implements $CloneEvent_FailedCopyWith<$Res> {
  _$CloneEvent_FailedCopyWithImpl(this._self, this._then);

  final CloneEvent_Failed _self;
  final $Res Function(CloneEvent_Failed) _then;

/// Create a copy of CloneEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? failure = null,}) {
  return _then(CloneEvent_Failed(
failure: null == failure ? _self.failure : failure // ignore: cast_nullable_to_non_nullable
as Failure,
  ));
}


}

/// @nodoc
mixin _$IndexEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is IndexEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'IndexEvent()';
}


}

/// @nodoc
class $IndexEventCopyWith<$Res>  {
$IndexEventCopyWith(IndexEvent _, $Res Function(IndexEvent) __);
}


/// Adds pattern-matching-related methods to [IndexEvent].
extension IndexEventPatterns on IndexEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( IndexEvent_Progress value)?  progress,TResult Function( IndexEvent_Done value)?  done,TResult Function( IndexEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case IndexEvent_Progress() when progress != null:
return progress(_that);case IndexEvent_Done() when done != null:
return done(_that);case IndexEvent_Failed() when failed != null:
return failed(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( IndexEvent_Progress value)  progress,required TResult Function( IndexEvent_Done value)  done,required TResult Function( IndexEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case IndexEvent_Progress():
return progress(_that);case IndexEvent_Done():
return done(_that);case IndexEvent_Failed():
return failed(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( IndexEvent_Progress value)?  progress,TResult? Function( IndexEvent_Done value)?  done,TResult? Function( IndexEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case IndexEvent_Progress() when progress != null:
return progress(_that);case IndexEvent_Done() when done != null:
return done(_that);case IndexEvent_Failed() when failed != null:
return failed(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( IndexProgress progress)?  progress,TResult Function( IndexStats stats)?  done,TResult Function( Failure failure)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case IndexEvent_Progress() when progress != null:
return progress(_that.progress);case IndexEvent_Done() when done != null:
return done(_that.stats);case IndexEvent_Failed() when failed != null:
return failed(_that.failure);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( IndexProgress progress)  progress,required TResult Function( IndexStats stats)  done,required TResult Function( Failure failure)  failed,}) {final _that = this;
switch (_that) {
case IndexEvent_Progress():
return progress(_that.progress);case IndexEvent_Done():
return done(_that.stats);case IndexEvent_Failed():
return failed(_that.failure);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( IndexProgress progress)?  progress,TResult? Function( IndexStats stats)?  done,TResult? Function( Failure failure)?  failed,}) {final _that = this;
switch (_that) {
case IndexEvent_Progress() when progress != null:
return progress(_that.progress);case IndexEvent_Done() when done != null:
return done(_that.stats);case IndexEvent_Failed() when failed != null:
return failed(_that.failure);case _:
  return null;

}
}

}

/// @nodoc


class IndexEvent_Progress extends IndexEvent {
  const IndexEvent_Progress({required this.progress}): super._();
  

 final  IndexProgress progress;

/// Create a copy of IndexEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$IndexEvent_ProgressCopyWith<IndexEvent_Progress> get copyWith => _$IndexEvent_ProgressCopyWithImpl<IndexEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is IndexEvent_Progress&&(identical(other.progress, progress) || other.progress == progress));
}


@override
int get hashCode {
    return Object.hash(runtimeType,progress);
}

@override
String toString() {
    return 'IndexEvent.progress(progress: $progress)';
}


}

/// @nodoc
abstract mixin class $IndexEvent_ProgressCopyWith<$Res> implements $IndexEventCopyWith<$Res> {
  factory $IndexEvent_ProgressCopyWith(IndexEvent_Progress value, $Res Function(IndexEvent_Progress) _then) = _$IndexEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 IndexProgress progress
});




}
/// @nodoc
class _$IndexEvent_ProgressCopyWithImpl<$Res>
    implements $IndexEvent_ProgressCopyWith<$Res> {
  _$IndexEvent_ProgressCopyWithImpl(this._self, this._then);

  final IndexEvent_Progress _self;
  final $Res Function(IndexEvent_Progress) _then;

/// Create a copy of IndexEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? progress = null,}) {
  return _then(IndexEvent_Progress(
progress: null == progress ? _self.progress : progress // ignore: cast_nullable_to_non_nullable
as IndexProgress,
  ));
}


}

/// @nodoc


class IndexEvent_Done extends IndexEvent {
  const IndexEvent_Done({required this.stats}): super._();
  

 final  IndexStats stats;

/// Create a copy of IndexEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$IndexEvent_DoneCopyWith<IndexEvent_Done> get copyWith => _$IndexEvent_DoneCopyWithImpl<IndexEvent_Done>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is IndexEvent_Done&&(identical(other.stats, stats) || other.stats == stats));
}


@override
int get hashCode {
    return Object.hash(runtimeType,stats);
}

@override
String toString() {
    return 'IndexEvent.done(stats: $stats)';
}


}

/// @nodoc
abstract mixin class $IndexEvent_DoneCopyWith<$Res> implements $IndexEventCopyWith<$Res> {
  factory $IndexEvent_DoneCopyWith(IndexEvent_Done value, $Res Function(IndexEvent_Done) _then) = _$IndexEvent_DoneCopyWithImpl;
@useResult
$Res call({
 IndexStats stats
});




}
/// @nodoc
class _$IndexEvent_DoneCopyWithImpl<$Res>
    implements $IndexEvent_DoneCopyWith<$Res> {
  _$IndexEvent_DoneCopyWithImpl(this._self, this._then);

  final IndexEvent_Done _self;
  final $Res Function(IndexEvent_Done) _then;

/// Create a copy of IndexEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? stats = null,}) {
  return _then(IndexEvent_Done(
stats: null == stats ? _self.stats : stats // ignore: cast_nullable_to_non_nullable
as IndexStats,
  ));
}


}

/// @nodoc


class IndexEvent_Failed extends IndexEvent {
  const IndexEvent_Failed({required this.failure}): super._();
  

 final  Failure failure;

/// Create a copy of IndexEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$IndexEvent_FailedCopyWith<IndexEvent_Failed> get copyWith => _$IndexEvent_FailedCopyWithImpl<IndexEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is IndexEvent_Failed&&(identical(other.failure, failure) || other.failure == failure));
}


@override
int get hashCode {
    return Object.hash(runtimeType,failure);
}

@override
String toString() {
    return 'IndexEvent.failed(failure: $failure)';
}


}

/// @nodoc
abstract mixin class $IndexEvent_FailedCopyWith<$Res> implements $IndexEventCopyWith<$Res> {
  factory $IndexEvent_FailedCopyWith(IndexEvent_Failed value, $Res Function(IndexEvent_Failed) _then) = _$IndexEvent_FailedCopyWithImpl;
@useResult
$Res call({
 Failure failure
});




}
/// @nodoc
class _$IndexEvent_FailedCopyWithImpl<$Res>
    implements $IndexEvent_FailedCopyWith<$Res> {
  _$IndexEvent_FailedCopyWithImpl(this._self, this._then);

  final IndexEvent_Failed _self;
  final $Res Function(IndexEvent_Failed) _then;

/// Create a copy of IndexEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? failure = null,}) {
  return _then(IndexEvent_Failed(
failure: null == failure ? _self.failure : failure // ignore: cast_nullable_to_non_nullable
as Failure,
  ));
}


}

/// @nodoc
mixin _$SyncEvent {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'SyncEvent()';
}


}

/// @nodoc
class $SyncEventCopyWith<$Res>  {
$SyncEventCopyWith(SyncEvent _, $Res Function(SyncEvent) __);
}


/// Adds pattern-matching-related methods to [SyncEvent].
extension SyncEventPatterns on SyncEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SyncEvent_Progress value)?  progress,TResult Function( SyncEvent_Done value)?  done,TResult Function( SyncEvent_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SyncEvent_Progress() when progress != null:
return progress(_that);case SyncEvent_Done() when done != null:
return done(_that);case SyncEvent_Failed() when failed != null:
return failed(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SyncEvent_Progress value)  progress,required TResult Function( SyncEvent_Done value)  done,required TResult Function( SyncEvent_Failed value)  failed,}){
final _that = this;
switch (_that) {
case SyncEvent_Progress():
return progress(_that);case SyncEvent_Done():
return done(_that);case SyncEvent_Failed():
return failed(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SyncEvent_Progress value)?  progress,TResult? Function( SyncEvent_Done value)?  done,TResult? Function( SyncEvent_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case SyncEvent_Progress() when progress != null:
return progress(_that);case SyncEvent_Done() when done != null:
return done(_that);case SyncEvent_Failed() when failed != null:
return failed(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( GitProgress progress)?  progress,TResult Function( SyncReport report)?  done,TResult Function( Failure failure)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SyncEvent_Progress() when progress != null:
return progress(_that.progress);case SyncEvent_Done() when done != null:
return done(_that.report);case SyncEvent_Failed() when failed != null:
return failed(_that.failure);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( GitProgress progress)  progress,required TResult Function( SyncReport report)  done,required TResult Function( Failure failure)  failed,}) {final _that = this;
switch (_that) {
case SyncEvent_Progress():
return progress(_that.progress);case SyncEvent_Done():
return done(_that.report);case SyncEvent_Failed():
return failed(_that.failure);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( GitProgress progress)?  progress,TResult? Function( SyncReport report)?  done,TResult? Function( Failure failure)?  failed,}) {final _that = this;
switch (_that) {
case SyncEvent_Progress() when progress != null:
return progress(_that.progress);case SyncEvent_Done() when done != null:
return done(_that.report);case SyncEvent_Failed() when failed != null:
return failed(_that.failure);case _:
  return null;

}
}

}

/// @nodoc


class SyncEvent_Progress extends SyncEvent {
  const SyncEvent_Progress({required this.progress}): super._();
  

 final  GitProgress progress;

/// Create a copy of SyncEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEvent_ProgressCopyWith<SyncEvent_Progress> get copyWith => _$SyncEvent_ProgressCopyWithImpl<SyncEvent_Progress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEvent_Progress&&(identical(other.progress, progress) || other.progress == progress));
}


@override
int get hashCode {
    return Object.hash(runtimeType,progress);
}

@override
String toString() {
    return 'SyncEvent.progress(progress: $progress)';
}


}

/// @nodoc
abstract mixin class $SyncEvent_ProgressCopyWith<$Res> implements $SyncEventCopyWith<$Res> {
  factory $SyncEvent_ProgressCopyWith(SyncEvent_Progress value, $Res Function(SyncEvent_Progress) _then) = _$SyncEvent_ProgressCopyWithImpl;
@useResult
$Res call({
 GitProgress progress
});




}
/// @nodoc
class _$SyncEvent_ProgressCopyWithImpl<$Res>
    implements $SyncEvent_ProgressCopyWith<$Res> {
  _$SyncEvent_ProgressCopyWithImpl(this._self, this._then);

  final SyncEvent_Progress _self;
  final $Res Function(SyncEvent_Progress) _then;

/// Create a copy of SyncEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? progress = null,}) {
  return _then(SyncEvent_Progress(
progress: null == progress ? _self.progress : progress // ignore: cast_nullable_to_non_nullable
as GitProgress,
  ));
}


}

/// @nodoc


class SyncEvent_Done extends SyncEvent {
  const SyncEvent_Done({required this.report}): super._();
  

 final  SyncReport report;

/// Create a copy of SyncEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEvent_DoneCopyWith<SyncEvent_Done> get copyWith => _$SyncEvent_DoneCopyWithImpl<SyncEvent_Done>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEvent_Done&&(identical(other.report, report) || other.report == report));
}


@override
int get hashCode {
    return Object.hash(runtimeType,report);
}

@override
String toString() {
    return 'SyncEvent.done(report: $report)';
}


}

/// @nodoc
abstract mixin class $SyncEvent_DoneCopyWith<$Res> implements $SyncEventCopyWith<$Res> {
  factory $SyncEvent_DoneCopyWith(SyncEvent_Done value, $Res Function(SyncEvent_Done) _then) = _$SyncEvent_DoneCopyWithImpl;
@useResult
$Res call({
 SyncReport report
});




}
/// @nodoc
class _$SyncEvent_DoneCopyWithImpl<$Res>
    implements $SyncEvent_DoneCopyWith<$Res> {
  _$SyncEvent_DoneCopyWithImpl(this._self, this._then);

  final SyncEvent_Done _self;
  final $Res Function(SyncEvent_Done) _then;

/// Create a copy of SyncEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? report = null,}) {
  return _then(SyncEvent_Done(
report: null == report ? _self.report : report // ignore: cast_nullable_to_non_nullable
as SyncReport,
  ));
}


}

/// @nodoc


class SyncEvent_Failed extends SyncEvent {
  const SyncEvent_Failed({required this.failure}): super._();
  

 final  Failure failure;

/// Create a copy of SyncEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncEvent_FailedCopyWith<SyncEvent_Failed> get copyWith => _$SyncEvent_FailedCopyWithImpl<SyncEvent_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncEvent_Failed&&(identical(other.failure, failure) || other.failure == failure));
}


@override
int get hashCode {
    return Object.hash(runtimeType,failure);
}

@override
String toString() {
    return 'SyncEvent.failed(failure: $failure)';
}


}

/// @nodoc
abstract mixin class $SyncEvent_FailedCopyWith<$Res> implements $SyncEventCopyWith<$Res> {
  factory $SyncEvent_FailedCopyWith(SyncEvent_Failed value, $Res Function(SyncEvent_Failed) _then) = _$SyncEvent_FailedCopyWithImpl;
@useResult
$Res call({
 Failure failure
});




}
/// @nodoc
class _$SyncEvent_FailedCopyWithImpl<$Res>
    implements $SyncEvent_FailedCopyWith<$Res> {
  _$SyncEvent_FailedCopyWithImpl(this._self, this._then);

  final SyncEvent_Failed _self;
  final $Res Function(SyncEvent_Failed) _then;

/// Create a copy of SyncEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? failure = null,}) {
  return _then(SyncEvent_Failed(
failure: null == failure ? _self.failure : failure // ignore: cast_nullable_to_non_nullable
as Failure,
  ));
}


}

// dart format on
