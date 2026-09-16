// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'error.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$PattoError {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'PattoError()';
}


}

/// @nodoc
class $PattoErrorCopyWith<$Res>  {
$PattoErrorCopyWith(PattoError _, $Res Function(PattoError) __);
}


/// Adds pattern-matching-related methods to [PattoError].
extension PattoErrorPatterns on PattoError {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( PattoError_Io value)?  io,TResult Function( PattoError_NotFound value)?  notFound,TResult Function( PattoError_AlreadyExists value)?  alreadyExists,TResult Function( PattoError_InvalidName value)?  invalidName,TResult Function( PattoError_NoTaskAtRow value)?  noTaskAtRow,TResult Function( PattoError_IndexNotBuilt value)?  indexNotBuilt,TResult Function( PattoError_Git value)?  git,required TResult orElse(),}){
final _that = this;
switch (_that) {
case PattoError_Io() when io != null:
return io(_that);case PattoError_NotFound() when notFound != null:
return notFound(_that);case PattoError_AlreadyExists() when alreadyExists != null:
return alreadyExists(_that);case PattoError_InvalidName() when invalidName != null:
return invalidName(_that);case PattoError_NoTaskAtRow() when noTaskAtRow != null:
return noTaskAtRow(_that);case PattoError_IndexNotBuilt() when indexNotBuilt != null:
return indexNotBuilt(_that);case PattoError_Git() when git != null:
return git(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( PattoError_Io value)  io,required TResult Function( PattoError_NotFound value)  notFound,required TResult Function( PattoError_AlreadyExists value)  alreadyExists,required TResult Function( PattoError_InvalidName value)  invalidName,required TResult Function( PattoError_NoTaskAtRow value)  noTaskAtRow,required TResult Function( PattoError_IndexNotBuilt value)  indexNotBuilt,required TResult Function( PattoError_Git value)  git,}){
final _that = this;
switch (_that) {
case PattoError_Io():
return io(_that);case PattoError_NotFound():
return notFound(_that);case PattoError_AlreadyExists():
return alreadyExists(_that);case PattoError_InvalidName():
return invalidName(_that);case PattoError_NoTaskAtRow():
return noTaskAtRow(_that);case PattoError_IndexNotBuilt():
return indexNotBuilt(_that);case PattoError_Git():
return git(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( PattoError_Io value)?  io,TResult? Function( PattoError_NotFound value)?  notFound,TResult? Function( PattoError_AlreadyExists value)?  alreadyExists,TResult? Function( PattoError_InvalidName value)?  invalidName,TResult? Function( PattoError_NoTaskAtRow value)?  noTaskAtRow,TResult? Function( PattoError_IndexNotBuilt value)?  indexNotBuilt,TResult? Function( PattoError_Git value)?  git,}){
final _that = this;
switch (_that) {
case PattoError_Io() when io != null:
return io(_that);case PattoError_NotFound() when notFound != null:
return notFound(_that);case PattoError_AlreadyExists() when alreadyExists != null:
return alreadyExists(_that);case PattoError_InvalidName() when invalidName != null:
return invalidName(_that);case PattoError_NoTaskAtRow() when noTaskAtRow != null:
return noTaskAtRow(_that);case PattoError_IndexNotBuilt() when indexNotBuilt != null:
return indexNotBuilt(_that);case PattoError_Git() when git != null:
return git(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  io,TResult Function( String field0)?  notFound,TResult Function( String field0)?  alreadyExists,TResult Function( String field0)?  invalidName,TResult Function( String path,  int row)?  noTaskAtRow,TResult Function( String field0)?  indexNotBuilt,TResult Function( GitErrorKind kind,  String message)?  git,required TResult orElse(),}) {final _that = this;
switch (_that) {
case PattoError_Io() when io != null:
return io(_that.field0);case PattoError_NotFound() when notFound != null:
return notFound(_that.field0);case PattoError_AlreadyExists() when alreadyExists != null:
return alreadyExists(_that.field0);case PattoError_InvalidName() when invalidName != null:
return invalidName(_that.field0);case PattoError_NoTaskAtRow() when noTaskAtRow != null:
return noTaskAtRow(_that.path,_that.row);case PattoError_IndexNotBuilt() when indexNotBuilt != null:
return indexNotBuilt(_that.field0);case PattoError_Git() when git != null:
return git(_that.kind,_that.message);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  io,required TResult Function( String field0)  notFound,required TResult Function( String field0)  alreadyExists,required TResult Function( String field0)  invalidName,required TResult Function( String path,  int row)  noTaskAtRow,required TResult Function( String field0)  indexNotBuilt,required TResult Function( GitErrorKind kind,  String message)  git,}) {final _that = this;
switch (_that) {
case PattoError_Io():
return io(_that.field0);case PattoError_NotFound():
return notFound(_that.field0);case PattoError_AlreadyExists():
return alreadyExists(_that.field0);case PattoError_InvalidName():
return invalidName(_that.field0);case PattoError_NoTaskAtRow():
return noTaskAtRow(_that.path,_that.row);case PattoError_IndexNotBuilt():
return indexNotBuilt(_that.field0);case PattoError_Git():
return git(_that.kind,_that.message);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  io,TResult? Function( String field0)?  notFound,TResult? Function( String field0)?  alreadyExists,TResult? Function( String field0)?  invalidName,TResult? Function( String path,  int row)?  noTaskAtRow,TResult? Function( String field0)?  indexNotBuilt,TResult? Function( GitErrorKind kind,  String message)?  git,}) {final _that = this;
switch (_that) {
case PattoError_Io() when io != null:
return io(_that.field0);case PattoError_NotFound() when notFound != null:
return notFound(_that.field0);case PattoError_AlreadyExists() when alreadyExists != null:
return alreadyExists(_that.field0);case PattoError_InvalidName() when invalidName != null:
return invalidName(_that.field0);case PattoError_NoTaskAtRow() when noTaskAtRow != null:
return noTaskAtRow(_that.path,_that.row);case PattoError_IndexNotBuilt() when indexNotBuilt != null:
return indexNotBuilt(_that.field0);case PattoError_Git() when git != null:
return git(_that.kind,_that.message);case _:
  return null;

}
}

}

/// @nodoc


class PattoError_Io extends PattoError {
  const PattoError_Io(this.field0): super._();
  

 final  String field0;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_IoCopyWith<PattoError_Io> get copyWith => _$PattoError_IoCopyWithImpl<PattoError_Io>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_Io&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'PattoError.io(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PattoError_IoCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_IoCopyWith(PattoError_Io value, $Res Function(PattoError_Io) _then) = _$PattoError_IoCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$PattoError_IoCopyWithImpl<$Res>
    implements $PattoError_IoCopyWith<$Res> {
  _$PattoError_IoCopyWithImpl(this._self, this._then);

  final PattoError_Io _self;
  final $Res Function(PattoError_Io) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PattoError_Io(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PattoError_NotFound extends PattoError {
  const PattoError_NotFound(this.field0): super._();
  

 final  String field0;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_NotFoundCopyWith<PattoError_NotFound> get copyWith => _$PattoError_NotFoundCopyWithImpl<PattoError_NotFound>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_NotFound&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'PattoError.notFound(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PattoError_NotFoundCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_NotFoundCopyWith(PattoError_NotFound value, $Res Function(PattoError_NotFound) _then) = _$PattoError_NotFoundCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$PattoError_NotFoundCopyWithImpl<$Res>
    implements $PattoError_NotFoundCopyWith<$Res> {
  _$PattoError_NotFoundCopyWithImpl(this._self, this._then);

  final PattoError_NotFound _self;
  final $Res Function(PattoError_NotFound) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PattoError_NotFound(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PattoError_AlreadyExists extends PattoError {
  const PattoError_AlreadyExists(this.field0): super._();
  

 final  String field0;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_AlreadyExistsCopyWith<PattoError_AlreadyExists> get copyWith => _$PattoError_AlreadyExistsCopyWithImpl<PattoError_AlreadyExists>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_AlreadyExists&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'PattoError.alreadyExists(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PattoError_AlreadyExistsCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_AlreadyExistsCopyWith(PattoError_AlreadyExists value, $Res Function(PattoError_AlreadyExists) _then) = _$PattoError_AlreadyExistsCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$PattoError_AlreadyExistsCopyWithImpl<$Res>
    implements $PattoError_AlreadyExistsCopyWith<$Res> {
  _$PattoError_AlreadyExistsCopyWithImpl(this._self, this._then);

  final PattoError_AlreadyExists _self;
  final $Res Function(PattoError_AlreadyExists) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PattoError_AlreadyExists(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PattoError_InvalidName extends PattoError {
  const PattoError_InvalidName(this.field0): super._();
  

 final  String field0;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_InvalidNameCopyWith<PattoError_InvalidName> get copyWith => _$PattoError_InvalidNameCopyWithImpl<PattoError_InvalidName>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_InvalidName&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'PattoError.invalidName(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PattoError_InvalidNameCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_InvalidNameCopyWith(PattoError_InvalidName value, $Res Function(PattoError_InvalidName) _then) = _$PattoError_InvalidNameCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$PattoError_InvalidNameCopyWithImpl<$Res>
    implements $PattoError_InvalidNameCopyWith<$Res> {
  _$PattoError_InvalidNameCopyWithImpl(this._self, this._then);

  final PattoError_InvalidName _self;
  final $Res Function(PattoError_InvalidName) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PattoError_InvalidName(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PattoError_NoTaskAtRow extends PattoError {
  const PattoError_NoTaskAtRow({required this.path, required this.row}): super._();
  

 final  String path;
 final  int row;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_NoTaskAtRowCopyWith<PattoError_NoTaskAtRow> get copyWith => _$PattoError_NoTaskAtRowCopyWithImpl<PattoError_NoTaskAtRow>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_NoTaskAtRow&&(identical(other.path, path) || other.path == path)&&(identical(other.row, row) || other.row == row));
}


@override
int get hashCode {
    return Object.hash(runtimeType,path,row);
}

@override
String toString() {
    return 'PattoError.noTaskAtRow(path: $path, row: $row)';
}


}

/// @nodoc
abstract mixin class $PattoError_NoTaskAtRowCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_NoTaskAtRowCopyWith(PattoError_NoTaskAtRow value, $Res Function(PattoError_NoTaskAtRow) _then) = _$PattoError_NoTaskAtRowCopyWithImpl;
@useResult
$Res call({
 String path, int row
});




}
/// @nodoc
class _$PattoError_NoTaskAtRowCopyWithImpl<$Res>
    implements $PattoError_NoTaskAtRowCopyWith<$Res> {
  _$PattoError_NoTaskAtRowCopyWithImpl(this._self, this._then);

  final PattoError_NoTaskAtRow _self;
  final $Res Function(PattoError_NoTaskAtRow) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? path = null,Object? row = null,}) {
  return _then(PattoError_NoTaskAtRow(
path: null == path ? _self.path : path // ignore: cast_nullable_to_non_nullable
as String,row: null == row ? _self.row : row // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class PattoError_IndexNotBuilt extends PattoError {
  const PattoError_IndexNotBuilt(this.field0): super._();
  

 final  String field0;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_IndexNotBuiltCopyWith<PattoError_IndexNotBuilt> get copyWith => _$PattoError_IndexNotBuiltCopyWithImpl<PattoError_IndexNotBuilt>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_IndexNotBuilt&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'PattoError.indexNotBuilt(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $PattoError_IndexNotBuiltCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_IndexNotBuiltCopyWith(PattoError_IndexNotBuilt value, $Res Function(PattoError_IndexNotBuilt) _then) = _$PattoError_IndexNotBuiltCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$PattoError_IndexNotBuiltCopyWithImpl<$Res>
    implements $PattoError_IndexNotBuiltCopyWith<$Res> {
  _$PattoError_IndexNotBuiltCopyWithImpl(this._self, this._then);

  final PattoError_IndexNotBuilt _self;
  final $Res Function(PattoError_IndexNotBuilt) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(PattoError_IndexNotBuilt(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class PattoError_Git extends PattoError {
  const PattoError_Git({required this.kind, required this.message}): super._();
  

 final  GitErrorKind kind;
 final  String message;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PattoError_GitCopyWith<PattoError_Git> get copyWith => _$PattoError_GitCopyWithImpl<PattoError_Git>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is PattoError_Git&&(identical(other.kind, kind) || other.kind == kind)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode {
    return Object.hash(runtimeType,kind,message);
}

@override
String toString() {
    return 'PattoError.git(kind: $kind, message: $message)';
}


}

/// @nodoc
abstract mixin class $PattoError_GitCopyWith<$Res> implements $PattoErrorCopyWith<$Res> {
  factory $PattoError_GitCopyWith(PattoError_Git value, $Res Function(PattoError_Git) _then) = _$PattoError_GitCopyWithImpl;
@useResult
$Res call({
 GitErrorKind kind, String message
});




}
/// @nodoc
class _$PattoError_GitCopyWithImpl<$Res>
    implements $PattoError_GitCopyWith<$Res> {
  _$PattoError_GitCopyWithImpl(this._self, this._then);

  final PattoError_Git _self;
  final $Res Function(PattoError_Git) _then;

/// Create a copy of PattoError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? kind = null,Object? message = null,}) {
  return _then(PattoError_Git(
kind: null == kind ? _self.kind : kind // ignore: cast_nullable_to_non_nullable
as GitErrorKind,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
