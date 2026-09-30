// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'git.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$MergeOutcome {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeOutcome);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'MergeOutcome()';
}


}

/// @nodoc
class $MergeOutcomeCopyWith<$Res>  {
$MergeOutcomeCopyWith(MergeOutcome _, $Res Function(MergeOutcome) __);
}


/// Adds pattern-matching-related methods to [MergeOutcome].
extension MergeOutcomePatterns on MergeOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MergeOutcome_UpToDate value)?  upToDate,TResult Function( MergeOutcome_FastForward value)?  fastForward,TResult Function( MergeOutcome_Merged value)?  merged,TResult Function( MergeOutcome_Conflicted value)?  conflicted,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MergeOutcome_UpToDate() when upToDate != null:
return upToDate(_that);case MergeOutcome_FastForward() when fastForward != null:
return fastForward(_that);case MergeOutcome_Merged() when merged != null:
return merged(_that);case MergeOutcome_Conflicted() when conflicted != null:
return conflicted(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MergeOutcome_UpToDate value)  upToDate,required TResult Function( MergeOutcome_FastForward value)  fastForward,required TResult Function( MergeOutcome_Merged value)  merged,required TResult Function( MergeOutcome_Conflicted value)  conflicted,}){
final _that = this;
switch (_that) {
case MergeOutcome_UpToDate():
return upToDate(_that);case MergeOutcome_FastForward():
return fastForward(_that);case MergeOutcome_Merged():
return merged(_that);case MergeOutcome_Conflicted():
return conflicted(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MergeOutcome_UpToDate value)?  upToDate,TResult? Function( MergeOutcome_FastForward value)?  fastForward,TResult? Function( MergeOutcome_Merged value)?  merged,TResult? Function( MergeOutcome_Conflicted value)?  conflicted,}){
final _that = this;
switch (_that) {
case MergeOutcome_UpToDate() when upToDate != null:
return upToDate(_that);case MergeOutcome_FastForward() when fastForward != null:
return fastForward(_that);case MergeOutcome_Merged() when merged != null:
return merged(_that);case MergeOutcome_Conflicted() when conflicted != null:
return conflicted(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  upToDate,TResult Function()?  fastForward,TResult Function()?  merged,TResult Function( String sideBranch,  List<String> paths)?  conflicted,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MergeOutcome_UpToDate() when upToDate != null:
return upToDate();case MergeOutcome_FastForward() when fastForward != null:
return fastForward();case MergeOutcome_Merged() when merged != null:
return merged();case MergeOutcome_Conflicted() when conflicted != null:
return conflicted(_that.sideBranch,_that.paths);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  upToDate,required TResult Function()  fastForward,required TResult Function()  merged,required TResult Function( String sideBranch,  List<String> paths)  conflicted,}) {final _that = this;
switch (_that) {
case MergeOutcome_UpToDate():
return upToDate();case MergeOutcome_FastForward():
return fastForward();case MergeOutcome_Merged():
return merged();case MergeOutcome_Conflicted():
return conflicted(_that.sideBranch,_that.paths);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  upToDate,TResult? Function()?  fastForward,TResult? Function()?  merged,TResult? Function( String sideBranch,  List<String> paths)?  conflicted,}) {final _that = this;
switch (_that) {
case MergeOutcome_UpToDate() when upToDate != null:
return upToDate();case MergeOutcome_FastForward() when fastForward != null:
return fastForward();case MergeOutcome_Merged() when merged != null:
return merged();case MergeOutcome_Conflicted() when conflicted != null:
return conflicted(_that.sideBranch,_that.paths);case _:
  return null;

}
}

}

/// @nodoc


class MergeOutcome_UpToDate extends MergeOutcome {
  const MergeOutcome_UpToDate(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeOutcome_UpToDate);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'MergeOutcome.upToDate()';
}


}




/// @nodoc


class MergeOutcome_FastForward extends MergeOutcome {
  const MergeOutcome_FastForward(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeOutcome_FastForward);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'MergeOutcome.fastForward()';
}


}




/// @nodoc


class MergeOutcome_Merged extends MergeOutcome {
  const MergeOutcome_Merged(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeOutcome_Merged);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'MergeOutcome.merged()';
}


}




/// @nodoc


class MergeOutcome_Conflicted extends MergeOutcome {
  const MergeOutcome_Conflicted({required this.sideBranch, required  List<String> paths}): _paths = paths,super._();
  

 final  String sideBranch;
 final  List<String> _paths;
 List<String> get paths {
  if (_paths is EqualUnmodifiableListView) return _paths;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_paths);
}


/// Create a copy of MergeOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MergeOutcome_ConflictedCopyWith<MergeOutcome_Conflicted> get copyWith => _$MergeOutcome_ConflictedCopyWithImpl<MergeOutcome_Conflicted>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is MergeOutcome_Conflicted&&(identical(other.sideBranch, sideBranch) || other.sideBranch == sideBranch)&&const DeepCollectionEquality().equals(other.paths, _paths));
}


@override
int get hashCode {
    return Object.hash(runtimeType,sideBranch,const DeepCollectionEquality().hash(_paths));
}

@override
String toString() {
    return 'MergeOutcome.conflicted(sideBranch: $sideBranch, paths: $paths)';
}


}

/// @nodoc
abstract mixin class $MergeOutcome_ConflictedCopyWith<$Res> implements $MergeOutcomeCopyWith<$Res> {
  factory $MergeOutcome_ConflictedCopyWith(MergeOutcome_Conflicted value, $Res Function(MergeOutcome_Conflicted) _then) = _$MergeOutcome_ConflictedCopyWithImpl;
@useResult
$Res call({
 String sideBranch, List<String> paths
});




}
/// @nodoc
class _$MergeOutcome_ConflictedCopyWithImpl<$Res>
    implements $MergeOutcome_ConflictedCopyWith<$Res> {
  _$MergeOutcome_ConflictedCopyWithImpl(this._self, this._then);

  final MergeOutcome_Conflicted _self;
  final $Res Function(MergeOutcome_Conflicted) _then;

/// Create a copy of MergeOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? sideBranch = null,Object? paths = null,}) {
  return _then(MergeOutcome_Conflicted(
sideBranch: null == sideBranch ? _self.sideBranch : sideBranch // ignore: cast_nullable_to_non_nullable
as String,paths: null == paths ? _self._paths : paths // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

// dart format on
