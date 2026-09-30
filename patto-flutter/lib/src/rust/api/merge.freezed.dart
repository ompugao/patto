// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'merge.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$MergeRegion {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeRegion);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'MergeRegion()';
}


}

/// @nodoc
class $MergeRegionCopyWith<$Res>  {
$MergeRegionCopyWith(MergeRegion _, $Res Function(MergeRegion) __);
}


/// Adds pattern-matching-related methods to [MergeRegion].
extension MergeRegionPatterns on MergeRegion {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MergeRegion_Unchanged value)?  unchanged,TResult Function( MergeRegion_Ours value)?  ours,TResult Function( MergeRegion_Theirs value)?  theirs,TResult Function( MergeRegion_Same value)?  same,TResult Function( MergeRegion_Conflict value)?  conflict,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MergeRegion_Unchanged() when unchanged != null:
return unchanged(_that);case MergeRegion_Ours() when ours != null:
return ours(_that);case MergeRegion_Theirs() when theirs != null:
return theirs(_that);case MergeRegion_Same() when same != null:
return same(_that);case MergeRegion_Conflict() when conflict != null:
return conflict(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MergeRegion_Unchanged value)  unchanged,required TResult Function( MergeRegion_Ours value)  ours,required TResult Function( MergeRegion_Theirs value)  theirs,required TResult Function( MergeRegion_Same value)  same,required TResult Function( MergeRegion_Conflict value)  conflict,}){
final _that = this;
switch (_that) {
case MergeRegion_Unchanged():
return unchanged(_that);case MergeRegion_Ours():
return ours(_that);case MergeRegion_Theirs():
return theirs(_that);case MergeRegion_Same():
return same(_that);case MergeRegion_Conflict():
return conflict(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MergeRegion_Unchanged value)?  unchanged,TResult? Function( MergeRegion_Ours value)?  ours,TResult? Function( MergeRegion_Theirs value)?  theirs,TResult? Function( MergeRegion_Same value)?  same,TResult? Function( MergeRegion_Conflict value)?  conflict,}){
final _that = this;
switch (_that) {
case MergeRegion_Unchanged() when unchanged != null:
return unchanged(_that);case MergeRegion_Ours() when ours != null:
return ours(_that);case MergeRegion_Theirs() when theirs != null:
return theirs(_that);case MergeRegion_Same() when same != null:
return same(_that);case MergeRegion_Conflict() when conflict != null:
return conflict(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( List<String> lines)?  unchanged,TResult Function( List<String> base,  List<String> lines)?  ours,TResult Function( List<String> base,  List<String> lines)?  theirs,TResult Function( List<String> base,  List<String> lines)?  same,TResult Function( List<String> base,  List<String> ours,  List<String> theirs,  Suggestion? suggestion)?  conflict,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MergeRegion_Unchanged() when unchanged != null:
return unchanged(_that.lines);case MergeRegion_Ours() when ours != null:
return ours(_that.base,_that.lines);case MergeRegion_Theirs() when theirs != null:
return theirs(_that.base,_that.lines);case MergeRegion_Same() when same != null:
return same(_that.base,_that.lines);case MergeRegion_Conflict() when conflict != null:
return conflict(_that.base,_that.ours,_that.theirs,_that.suggestion);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( List<String> lines)  unchanged,required TResult Function( List<String> base,  List<String> lines)  ours,required TResult Function( List<String> base,  List<String> lines)  theirs,required TResult Function( List<String> base,  List<String> lines)  same,required TResult Function( List<String> base,  List<String> ours,  List<String> theirs,  Suggestion? suggestion)  conflict,}) {final _that = this;
switch (_that) {
case MergeRegion_Unchanged():
return unchanged(_that.lines);case MergeRegion_Ours():
return ours(_that.base,_that.lines);case MergeRegion_Theirs():
return theirs(_that.base,_that.lines);case MergeRegion_Same():
return same(_that.base,_that.lines);case MergeRegion_Conflict():
return conflict(_that.base,_that.ours,_that.theirs,_that.suggestion);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( List<String> lines)?  unchanged,TResult? Function( List<String> base,  List<String> lines)?  ours,TResult? Function( List<String> base,  List<String> lines)?  theirs,TResult? Function( List<String> base,  List<String> lines)?  same,TResult? Function( List<String> base,  List<String> ours,  List<String> theirs,  Suggestion? suggestion)?  conflict,}) {final _that = this;
switch (_that) {
case MergeRegion_Unchanged() when unchanged != null:
return unchanged(_that.lines);case MergeRegion_Ours() when ours != null:
return ours(_that.base,_that.lines);case MergeRegion_Theirs() when theirs != null:
return theirs(_that.base,_that.lines);case MergeRegion_Same() when same != null:
return same(_that.base,_that.lines);case MergeRegion_Conflict() when conflict != null:
return conflict(_that.base,_that.ours,_that.theirs,_that.suggestion);case _:
  return null;

}
}

}

/// @nodoc


class MergeRegion_Unchanged extends MergeRegion {
  const MergeRegion_Unchanged({required  List<String> lines}): _lines = lines,super._();
  

 final  List<String> _lines;
 List<String> get lines {
  if (_lines is EqualUnmodifiableListView) return _lines;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_lines);
}


/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergeRegion_UnchangedCopyWith<MergeRegion_Unchanged> get copyWith => _$MergeRegion_UnchangedCopyWithImpl<MergeRegion_Unchanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeRegion_Unchanged&&const DeepCollectionEquality().equals(other.lines, _lines));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_lines));
}

@override
String toString() {
    return 'MergeRegion.unchanged(lines: $lines)';
}


}

/// @nodoc
abstract mixin class $MergeRegion_UnchangedCopyWith<$Res> implements $MergeRegionCopyWith<$Res> {
  factory $MergeRegion_UnchangedCopyWith(MergeRegion_Unchanged value, $Res Function(MergeRegion_Unchanged) _then) = _$MergeRegion_UnchangedCopyWithImpl;
@useResult
$Res call({
 List<String> lines
});




}
/// @nodoc
class _$MergeRegion_UnchangedCopyWithImpl<$Res>
    implements $MergeRegion_UnchangedCopyWith<$Res> {
  _$MergeRegion_UnchangedCopyWithImpl(this._self, this._then);

  final MergeRegion_Unchanged _self;
  final $Res Function(MergeRegion_Unchanged) _then;

/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lines = null,}) {
  return _then(MergeRegion_Unchanged(
lines: null == lines ? _self._lines : lines // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class MergeRegion_Ours extends MergeRegion {
  const MergeRegion_Ours({required  List<String> base, required  List<String> lines}): _base = base,_lines = lines,super._();
  

 final  List<String> _base;
 List<String> get base {
  if (_base is EqualUnmodifiableListView) return _base;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_base);
}

 final  List<String> _lines;
 List<String> get lines {
  if (_lines is EqualUnmodifiableListView) return _lines;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_lines);
}


/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergeRegion_OursCopyWith<MergeRegion_Ours> get copyWith => _$MergeRegion_OursCopyWithImpl<MergeRegion_Ours>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeRegion_Ours&&const DeepCollectionEquality().equals(other.base, _base)&&const DeepCollectionEquality().equals(other.lines, _lines));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_base),const DeepCollectionEquality().hash(_lines));
}

@override
String toString() {
    return 'MergeRegion.ours(base: $base, lines: $lines)';
}


}

/// @nodoc
abstract mixin class $MergeRegion_OursCopyWith<$Res> implements $MergeRegionCopyWith<$Res> {
  factory $MergeRegion_OursCopyWith(MergeRegion_Ours value, $Res Function(MergeRegion_Ours) _then) = _$MergeRegion_OursCopyWithImpl;
@useResult
$Res call({
 List<String> base, List<String> lines
});




}
/// @nodoc
class _$MergeRegion_OursCopyWithImpl<$Res>
    implements $MergeRegion_OursCopyWith<$Res> {
  _$MergeRegion_OursCopyWithImpl(this._self, this._then);

  final MergeRegion_Ours _self;
  final $Res Function(MergeRegion_Ours) _then;

/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? base = null,Object? lines = null,}) {
  return _then(MergeRegion_Ours(
base: null == base ? _self._base : base // ignore: cast_nullable_to_non_nullable
as List<String>,lines: null == lines ? _self._lines : lines // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class MergeRegion_Theirs extends MergeRegion {
  const MergeRegion_Theirs({required  List<String> base, required  List<String> lines}): _base = base,_lines = lines,super._();
  

 final  List<String> _base;
 List<String> get base {
  if (_base is EqualUnmodifiableListView) return _base;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_base);
}

 final  List<String> _lines;
 List<String> get lines {
  if (_lines is EqualUnmodifiableListView) return _lines;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_lines);
}


/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergeRegion_TheirsCopyWith<MergeRegion_Theirs> get copyWith => _$MergeRegion_TheirsCopyWithImpl<MergeRegion_Theirs>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeRegion_Theirs&&const DeepCollectionEquality().equals(other.base, _base)&&const DeepCollectionEquality().equals(other.lines, _lines));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_base),const DeepCollectionEquality().hash(_lines));
}

@override
String toString() {
    return 'MergeRegion.theirs(base: $base, lines: $lines)';
}


}

/// @nodoc
abstract mixin class $MergeRegion_TheirsCopyWith<$Res> implements $MergeRegionCopyWith<$Res> {
  factory $MergeRegion_TheirsCopyWith(MergeRegion_Theirs value, $Res Function(MergeRegion_Theirs) _then) = _$MergeRegion_TheirsCopyWithImpl;
@useResult
$Res call({
 List<String> base, List<String> lines
});




}
/// @nodoc
class _$MergeRegion_TheirsCopyWithImpl<$Res>
    implements $MergeRegion_TheirsCopyWith<$Res> {
  _$MergeRegion_TheirsCopyWithImpl(this._self, this._then);

  final MergeRegion_Theirs _self;
  final $Res Function(MergeRegion_Theirs) _then;

/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? base = null,Object? lines = null,}) {
  return _then(MergeRegion_Theirs(
base: null == base ? _self._base : base // ignore: cast_nullable_to_non_nullable
as List<String>,lines: null == lines ? _self._lines : lines // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class MergeRegion_Same extends MergeRegion {
  const MergeRegion_Same({required  List<String> base, required  List<String> lines}): _base = base,_lines = lines,super._();
  

 final  List<String> _base;
 List<String> get base {
  if (_base is EqualUnmodifiableListView) return _base;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_base);
}

 final  List<String> _lines;
 List<String> get lines {
  if (_lines is EqualUnmodifiableListView) return _lines;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_lines);
}


/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergeRegion_SameCopyWith<MergeRegion_Same> get copyWith => _$MergeRegion_SameCopyWithImpl<MergeRegion_Same>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeRegion_Same&&const DeepCollectionEquality().equals(other.base, _base)&&const DeepCollectionEquality().equals(other.lines, _lines));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_base),const DeepCollectionEquality().hash(_lines));
}

@override
String toString() {
    return 'MergeRegion.same(base: $base, lines: $lines)';
}


}

/// @nodoc
abstract mixin class $MergeRegion_SameCopyWith<$Res> implements $MergeRegionCopyWith<$Res> {
  factory $MergeRegion_SameCopyWith(MergeRegion_Same value, $Res Function(MergeRegion_Same) _then) = _$MergeRegion_SameCopyWithImpl;
@useResult
$Res call({
 List<String> base, List<String> lines
});




}
/// @nodoc
class _$MergeRegion_SameCopyWithImpl<$Res>
    implements $MergeRegion_SameCopyWith<$Res> {
  _$MergeRegion_SameCopyWithImpl(this._self, this._then);

  final MergeRegion_Same _self;
  final $Res Function(MergeRegion_Same) _then;

/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? base = null,Object? lines = null,}) {
  return _then(MergeRegion_Same(
base: null == base ? _self._base : base // ignore: cast_nullable_to_non_nullable
as List<String>,lines: null == lines ? _self._lines : lines // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class MergeRegion_Conflict extends MergeRegion {
  const MergeRegion_Conflict({required  List<String> base, required  List<String> ours, required  List<String> theirs, this.suggestion}): _base = base,_ours = ours,_theirs = theirs,super._();
  

 final  List<String> _base;
 List<String> get base {
  if (_base is EqualUnmodifiableListView) return _base;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_base);
}

 final  List<String> _ours;
 List<String> get ours {
  if (_ours is EqualUnmodifiableListView) return _ours;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_ours);
}

 final  List<String> _theirs;
 List<String> get theirs {
  if (_theirs is EqualUnmodifiableListView) return _theirs;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_theirs);
}

 final  Suggestion? suggestion;

/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergeRegion_ConflictCopyWith<MergeRegion_Conflict> get copyWith => _$MergeRegion_ConflictCopyWithImpl<MergeRegion_Conflict>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeRegion_Conflict&&const DeepCollectionEquality().equals(other.base, _base)&&const DeepCollectionEquality().equals(other.ours, _ours)&&const DeepCollectionEquality().equals(other.theirs, _theirs)&&(identical(other.suggestion, suggestion) || other.suggestion == suggestion));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_base),const DeepCollectionEquality().hash(_ours),const DeepCollectionEquality().hash(_theirs),suggestion);
}

@override
String toString() {
    return 'MergeRegion.conflict(base: $base, ours: $ours, theirs: $theirs, suggestion: $suggestion)';
}


}

/// @nodoc
abstract mixin class $MergeRegion_ConflictCopyWith<$Res> implements $MergeRegionCopyWith<$Res> {
  factory $MergeRegion_ConflictCopyWith(MergeRegion_Conflict value, $Res Function(MergeRegion_Conflict) _then) = _$MergeRegion_ConflictCopyWithImpl;
@useResult
$Res call({
 List<String> base, List<String> ours, List<String> theirs, Suggestion? suggestion
});




}
/// @nodoc
class _$MergeRegion_ConflictCopyWithImpl<$Res>
    implements $MergeRegion_ConflictCopyWith<$Res> {
  _$MergeRegion_ConflictCopyWithImpl(this._self, this._then);

  final MergeRegion_Conflict _self;
  final $Res Function(MergeRegion_Conflict) _then;

/// Create a copy of MergeRegion
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? base = null,Object? ours = null,Object? theirs = null,Object? suggestion = freezed,}) {
  return _then(MergeRegion_Conflict(
base: null == base ? _self._base : base // ignore: cast_nullable_to_non_nullable
as List<String>,ours: null == ours ? _self._ours : ours // ignore: cast_nullable_to_non_nullable
as List<String>,theirs: null == theirs ? _self._theirs : theirs // ignore: cast_nullable_to_non_nullable
as List<String>,suggestion: freezed == suggestion ? _self.suggestion : suggestion // ignore: cast_nullable_to_non_nullable
as Suggestion?,
  ));
}


}

// dart format on
